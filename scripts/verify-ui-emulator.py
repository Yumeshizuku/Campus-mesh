#!/usr/bin/env python3
"""UI smoke test on a dedicated Android emulator, using the real Rust/TCP core.
Prerequisites: x86_64 debug APK installed, unlocked emulator, adb on PATH,
`cargo build --bin mesh-node`. No radio or physical-device claims are made.
Does not print private state. Test peer state is temporary and removed on exit.
"""
import argparse
import json
import os
import pathlib
import re
import select
import shlex
import socket
import subprocess
import tempfile
import time
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--reset', action='store_true', help='Clear this dedicated emulator app before testing')
args = parser.parse_args()
assert args.serial.startswith('emulator-'), 'Use a dedicated emulator, not a personal phone'
ADB = os.environ.get('ADB', 'adb')

def adb(*command):
    return subprocess.check_output([ADB, '-s', args.serial, *command], timeout=40)

def tree():
    adb('shell', 'uiautomator', 'dump', '/sdcard/campus-ui.xml')
    return ET.fromstring(adb('shell', 'cat', '/sdcard/campus-ui.xml'))

def nodes():
    return list(tree().iter('node'))

def click_node(node):
    left, top, right, bottom = map(int, re.findall(r'\d+', node.attrib['bounds']))
    adb('shell', 'input', 'tap', str((left+right)//2), str((top+bottom)//2))

def tap(text, prefix=False):
    for node in nodes():
        value=node.attrib.get('text','')
        if (value.startswith(text) if prefix else value==text) or node.attrib.get('content-desc')==text:
            click_node(node);time.sleep(.5);return
    raise AssertionError('UI element missing: '+text)

def edit(text,last=False):
    editable=[n for n in nodes() if n.attrib.get('class')=='android.widget.EditText']
    node=editable[-1] if last else editable[0]
    click_node(node)
    old=node.attrib.get('text','')
    adb('shell','input','keyevent','123')
    if old and node.attrib.get('hint')!=old:
        # Clear only the current editable value. Hints are not counted by Android.
        adb('shell','input','keyevent',*(['67']*min(len(old),200)))
    if text:adb('shell','input','text',shlex.quote(text.replace(' ','%s')))

def contains(text):
    return any(text in n.attrib.get('text','') for n in nodes())

def screenshot(name):
    (ROOT/'docs/ui'/name).write_bytes(adb('exec-out','screencap','-p'))

def hide_keyboard():
    adb('shell','input','keyevent','4');time.sleep(.4)

def wait_for(predicate,seconds=60):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        if predicate():return
        time.sleep(1)
    raise AssertionError('UI verification deadline exceeded')

class Peer:
    def __init__(self,path,port):
        self.log=open(path/'peer.log','w')
        self.p=subprocess.Popen([str(ROOT/'target/debug/mesh-node'),str(path/'peer.json'),'Lin',str(port)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.log,text=True,bufsize=1)
    def call(self,command):
        self.p.stdin.write(json.dumps(command)+'\n');self.p.stdin.flush()
        assert select.select([self.p.stdout],[],[],10)[0]
        response=json.loads(self.p.stdout.readline());assert response['ok'], response.get('error');return response['data']
    def close(self):
        self.p.stdin.write('quit\n');self.p.stdin.flush();self.p.wait(timeout=10);self.log.close()

results=[]
with tempfile.TemporaryDirectory(prefix='campus-ui-peer-') as temp:
    with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    peer=Peer(pathlib.Path(temp),port)
    try:
        if args.reset:adb('shell','pm','clear','org.campusmesh')
        adb('shell','am','start','-n','org.campusmesh/.MainActivity')
        time.sleep(2)
        if contains('欢迎来到 Campus Mesh'):
            edit('CampusTester');tap('开始使用')
        wait_for(lambda:contains('本机已就绪'))
        # Read the app's own emulator state privately; export only public identity.
        state=json.loads(adb('shell','run-as','org.campusmesh','cat','files/mesh.json'))
        identity=state['identity'];del state
        my_id=peer.call({'op':'contact','contact':identity})['id']
        public=peer.call({'op':'state'})['identity']
        tap('附近');tap('粘贴公钥文本')
        edit(json.dumps(public,separators=(',',':')));tap('添加')
        wait_for(lambda:contains('Lin'))
        tap('返回会话列表');tap('附近');tap('连接局域网地址');edit(f'10.0.2.2:{port}');tap('连接')
        tap('聊天');tap('Lin');edit('Saved_draft_123',last=True);hide_keyboard()
        tap('返回会话列表');tap('附近');tap('聊天');tap('Lin');assert contains('Saved_draft_123');results.append('draft_survives_tab_switch')
        tap('返回会话列表');tap('自定义');tap('外观 ·',prefix=True);tap('深色');wait_for(lambda:contains('外观 · 深色'))
        screenshot('settings-dark.png');results.append('dark_theme_applied')
        tap('聊天');tap('Lin');assert contains('Saved_draft_123')
        adb('shell','am','force-stop','org.campusmesh')
        adb('shell','am','start','-n','org.campusmesh/.MainActivity');wait_for(lambda:contains('本机已就绪'))
        tap('Lin');assert contains('Saved_draft_123');results.append('draft_survives_process_restart')
        tap('发送消息')
        wait_for(lambda:any(m['incoming'] and m['text']=='Saved_draft_123' for m in peer.call({'op':'state'})['history']))
        peer.call({'op':'send','to':my_id,'text':'Hello_from_Lin'})
        wait_for(lambda:contains('Hello_from_Lin'))
        screenshot('chat-dark.png');results.append('real_tcp_send_receive_via_ui')
        assert not any(n.attrib.get('class')=='android.widget.EditText' and n.attrib.get('text')=='Saved_draft_123' for n in nodes())
        results.append('composer_cleared_after_success')
        tap('返回会话列表');screenshot('threads-dark.png');tap('附近');screenshot('nearby-dark.png')
        tap('自定义');tap('外观 ·',prefix=True);tap('浅色');wait_for(lambda:contains('外观 · 浅色'))
        screenshot('settings-light.png');tap('聊天');screenshot('threads-light.png');tap('Lin');screenshot('chat-light.png')
        report={'result':'PASS','platform':'Android API 35 x86_64 emulator','transport':'real Rust TCP, emulator to localhost peer','checks':results,'radio_or_physical_device_test':False}
        (ROOT/'docs/reports/ui-emulator.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
        print(json.dumps(report,ensure_ascii=False))
    finally:peer.close()
