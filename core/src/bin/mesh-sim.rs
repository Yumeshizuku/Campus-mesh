//! Explicit protocol simulation, never used by the mobile production path.
use campus_mesh::Engine;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
fn main() {
    for n in [10usize, 30, 100, 300] {
        let mut random = StdRng::seed_from_u64(42 + n as u64);
        let mut nodes: Vec<_> = (0..n)
            .map(|i| Engine::new(&format!("virtual-{i}")))
            .collect();
        let sent = n.min(30);
        for i in 0..sent {
            let dest = (i + n / 2) % n;
            let a = nodes[i].identity.clone();
            let b = nodes[dest].identity.clone();
            nodes[i].add_contact(b.clone()).unwrap();
            nodes[dest].add_contact(a).unwrap();
            nodes[i]
                .send(&b.id(), &format!("message-{i}"), None, 1000)
                .unwrap();
        }
        let mut transfers = 0usize;
        let mut dropped = 0usize;
        for tick in 0..40 {
            let now = 1001 + tick * 10;
            // Two disconnected components for 120 virtual seconds, followed by
            // moving random encounters across the reunited population.
            let groups = if tick < 12 {
                vec![(0, n / 2), (n / 2, n)]
            } else {
                vec![(0, n)]
            };
            for (lo, hi) in groups {
                let mut order: Vec<_> = (lo..hi).collect();
                order.shuffle(&mut random);
                for pair in order.as_chunks::<2>().0 {
                    let a = pair[0];
                    let b = pair[1];
                    let from_a = nodes[a].outgoing(now);
                    let from_b = nodes[b].outgoing(now);
                    for (to, frames) in [(b, from_a), (a, from_b)] {
                        for frame in frames {
                            if nodes[to].knows(&frame.envelope.id) {
                                continue;
                            }
                            transfers += 1;
                            if random.gen_bool(0.1) {
                                dropped += 1;
                                continue;
                            }
                            let _ = nodes[to].receive(frame, now);
                        }
                    }
                }
            }
            // Restart one randomly selected node with precisely its durable state.
            if tick == 10 || tick == 25 {
                let index = random.gen_range(0..n);
                nodes[index] =
                    serde_json::from_slice(&serde_json::to_vec(&nodes[index]).unwrap()).unwrap();
            }
        }
        let delivered = nodes
            .iter()
            .flat_map(|e| e.history.iter())
            .filter(|m| !m.incoming && m.status == "delivered")
            .count();
        let received = nodes
            .iter()
            .flat_map(|e| e.history.iter())
            .filter(|m| m.incoming)
            .count();
        assert!(
            received * 10 >= sent * 9 && delivered * 10 >= sent * 9,
            "scenario delivery/ACK below 90% acceptance threshold"
        );
        println!(
            "{}",
            serde_json::json!({"nodes":n,"seed":42+n,"messages":sent,"received":received,"end_to_end_acked":delivered,"delivery_ratio":received as f64/sent as f64,"ack_ratio":delivered as f64/sent as f64,"virtual_seconds":400,"partition_seconds":120,"random_loss_probability":0.1,"attempted_transfers":transfers,"dropped_transfers":dropped,"state_restarts":2,"simulation_only":true})
        );
    }
}
