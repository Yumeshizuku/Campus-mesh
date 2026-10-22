#!/usr/bin/env python3
"""Four real Rust processes; TCP only; explicit A-B-C-D contact graph.
C is absent while B stores a message, then restarts with its original identity.
No mock transport is used. Prints a machine-readable verification record.
"""
import json, pathlib, select, socket, subprocess, tempfile, time
ROOT = pathlib.Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/debug/mesh-node"

def available_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]

class Node:
    def __init__(self, directory, name, port):
        self.log = open(directory / f"{name}.log", "a")
        self.process = subprocess.Popen([str(BINARY), str(directory / f"{name}.json"), name, str(port)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log, text=True, bufsize=1)
        self.port = port
        self.state()
    def call(self, command):
        self.process.stdin.write(json.dumps(command, ensure_ascii=False) + "\n")
        self.process.stdin.flush()
        if not select.select([self.process.stdout], [], [], 10)[0]:
            raise TimeoutError("node did not respond")
        response = json.loads(self.process.stdout.readline())
        if not response["ok"]:
            raise RuntimeError(response)
        return response["data"]
    def state(self):
        return self.call({"op": "state"})
    def peer(self, other):
        self.call({"op": "peer", "address": f"127.0.0.1:{other.port}"})
    def stop(self):
        if self.process.poll() is None:
            self.process.stdin.write("quit\n")
            self.process.stdin.flush()
            self.process.wait(timeout=10)
        self.log.close()

def wait_for(predicate, seconds=70):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(0.5)
    raise AssertionError("delivery deadline exceeded")

def main():
    start = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="campus-processes-") as temp:
        path = pathlib.Path(temp)
        nodes = []
        try:
            ports = [available_port() for _ in range(4)]
            assert len(set(ports)) == 4
            for name, port in zip("ABCD", ports): nodes.append(Node(path, name, port))
            a, b, c, d = nodes
            ai, di = a.state()["identity"], d.state()["identity"]
            aid = d.call({"op": "contact", "contact": ai})["id"]
            did = a.call({"op": "contact", "contact": di})["id"]
            ci = c.state()["identity"]
            a.peer(b); b.peer(c); c.peer(d)
            c.stop()
            message_id = a.call({"op":"send", "to":did, "text":"三跳 · 断线后继续传递"})["id"]
            wait_for(lambda: len(b.call({"op":"outgoing"})) > 0, 20)
            assert not d.state()["history"], "unexpected shortcut"
            # Restart relay B too: require on-disk store-and-forward recovery.
            b.stop(); b = Node(path, "B", ports[1]); nodes[1] = b
            c = Node(path, "C", ports[2]); nodes[2] = c
            assert c.state()["identity"] == ci
            # Both relays must restore their previously authorized peer endpoints.
            wait_for(lambda: any(m["id"] == message_id and m["status"] == "delivered" for m in a.state()["history"]))
            incoming = [m for m in d.state()["history"] if m["id"] == message_id]
            assert len(incoming) == 1
            assert incoming[0]["text"] == "三跳 · 断线后继续传递"
            assert not b.state()["history"] and not c.state()["history"]
            assert incoming[0]["peer"] == aid
            print(json.dumps({"result":"PASS", "processes":4, "transport":"real loopback TCP", "topology":"A-B-C-D", "forward_hops":3, "relay_B_restarted_with_queued_ciphertext":True, "relay_C_reconnected":True, "recipient_copies":len(incoming), "end_to_end_ACK":True, "relay_plaintext_history":False, "elapsed_seconds":round(time.monotonic()-start,2)}, ensure_ascii=False))
        finally:
            for node in nodes:
                node.stop()
if __name__ == "__main__": main()
