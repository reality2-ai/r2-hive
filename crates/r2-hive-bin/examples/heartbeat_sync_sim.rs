//! Heartbeat-sync host prototype — pulse-coupled oscillators ("fireflies") over a
//! MODELED half-duplex, lossy LoRa channel. Apparatus-first: prove the concept +
//! tune ε / jitter / T BEFORE the radios exist (see `docs/lora-heartbeat-sync-design.md`).
//!
//! Falsifiable claims (supervisor):
//!   (1) the LED PHASE converges to synchrony across a trust group, AND
//!   (2) collision-avoidance REQUIRES TX jitter — the naive (no-jitter) variant
//!       FAILS on a real half-duplex radio: synchronised firing → simultaneous TX
//!       → mutual deafness + collisions → clock drift wins → desync.
//! Plus: a partition splits the rhythm into two; a heal re-merges it.
//!
//! The radio model captures the essential physics:
//!   - finite airtime per fire-announce (a frame occupies the channel),
//!   - COLLISION: two same-group transmissions overlapping in time → both lost,
//!   - HALF-DUPLEX: a node cannot receive while it is transmitting (deaf),
//!   - clock DRIFT: each node's period is off by a small per-node amount (this is
//!     what exposes the naive variant — synced-but-deaf nodes can't self-correct).
//!
//! Run:  cargo run -p r2-hive --example heartbeat_sync_sim --release
//! Deterministic (seeded LCG) — reproducible, no `rand` dependency.

/// Tiny deterministic PRNG (reproducible jitter/drift; no external dep).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    /// Uniform in [0, 1).
    fn f01(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / ((1u64 << 24) as f32)
    }
    /// Uniform in [-1, 1).
    fn pm1(&mut self) -> f32 {
        self.f01() * 2.0 - 1.0
    }
}

const DT: f32 = 0.005; // 5 ms simulation step
const AIRTIME: f32 = 0.06; // 60 ms time-on-air for a tiny SF7 fire frame

#[derive(Clone)]
struct Node {
    phase: f32,    // [0,1); LED fires at 1.0
    period: f32,   // nominal T + per-node clock drift
    group: u8,     // partition cluster (same group = in radio range)
    pending: bool, // fired, waiting to actually transmit (after jitter)
    tx_at: f32,    // scheduled TX start time
    tx_end: f32,   // >0 while transmitting (until this time); deaf meanwhile
    collided: bool, // this in-flight TX overlapped another → lost
}

struct Config {
    n: usize,
    period: f32,    // nominal heartbeat T
    eps: f32,       // coupling strength (phase advance on hearing a fire)
    jitter: f32,    // TX jitter as a fraction of T (0 = naive)
    drift: f32,     // per-node clock drift, ± fraction
    duration: f32,  // seconds
}

struct Stats {
    final_r: f32,       // phase order parameter at the end (1 = perfect sync)
    mean_late_r: f32,   // mean R over the last third (steady-state sync quality)
    collisions: u64,
    deliveries: u64,
}

/// Phase order parameter R = |mean(e^{i 2π φ})| over a node set. 1 = synchronised.
fn order_parameter(nodes: &[&Node]) -> f32 {
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    for nd in nodes {
        let a = core::f32::consts::TAU * nd.phase;
        sx += a.cos();
        sy += a.sin();
    }
    let n = nodes.len() as f32;
    ((sx / n).powi(2) + (sy / n).powi(2)).sqrt()
}

/// Run one scenario. `repartition` may move nodes between groups at given times
/// (to inject a partition and a later heal).
fn run(cfg: &Config, seed: u64, repartition: &[(f32, fn(usize) -> u8)]) -> (Stats, Vec<(f32, f32)>) {
    let mut rng = Lcg(seed);
    let mut nodes: Vec<Node> = (0..cfg.n)
        .map(|_| Node {
            phase: rng.f01(), // random initial phases
            period: cfg.period * (1.0 + cfg.drift * rng.pm1()),
            group: 0,
            pending: false,
            tx_at: 0.0,
            tx_end: 0.0,
            collided: false,
        })
        .collect();

    let mut collisions = 0u64;
    let mut deliveries = 0u64;
    let mut r_series: Vec<(f32, f32)> = Vec::new();
    let mut late_r_sum = 0.0f32;
    let mut late_r_n = 0u32;
    let late_start = cfg.duration * 2.0 / 3.0;

    let steps = (cfg.duration / DT) as usize;
    let mut next_repartition = 0usize;

    let mut t = 0.0f32;
    for step in 0..steps {
        // Apply scheduled repartition (partition / heal).
        while next_repartition < repartition.len() && t >= repartition[next_repartition].0 {
            let f = repartition[next_repartition].1;
            for (i, nd) in nodes.iter_mut().enumerate() {
                nd.group = f(i);
            }
            next_repartition += 1;
        }

        // 1. Advance phase; fire on wrap.
        for nd in nodes.iter_mut() {
            nd.phase += DT / nd.period;
            if nd.phase >= 1.0 {
                nd.phase -= 1.0;
                // Schedule the radio announce: immediate (naive) or jittered.
                nd.pending = true;
                nd.tx_at = t + cfg.jitter * cfg.period * rng_frac(&mut rng);
            }
        }

        // 2. Begin transmissions whose scheduled time has arrived.
        for nd in nodes.iter_mut() {
            if nd.pending && nd.tx_end == 0.0 && nd.tx_at <= t {
                nd.tx_end = t + AIRTIME;
                nd.collided = false;
                nd.pending = false;
            }
        }

        // 3. Collision detection: ≥2 active TX in the same group → all lost.
        for g in 0..=max_group(&nodes) {
            let active: Vec<usize> = nodes
                .iter()
                .enumerate()
                .filter(|(_, nd)| nd.tx_end > t && nd.group == g)
                .map(|(i, _)| i)
                .collect();
            if active.len() >= 2 {
                for &i in &active {
                    nodes[i].collided = true;
                }
            }
        }

        // 4. Completing transmissions deliver a fire-pulse (unless collided), to
        //    same-group nodes that are NOT transmitting (half-duplex deaf).
        let completing: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, nd)| nd.tx_end > 0.0 && nd.tx_end <= t)
            .map(|(i, _)| i)
            .collect();
        for i in completing {
            let (g, lost) = (nodes[i].group, nodes[i].collided);
            nodes[i].tx_end = 0.0;
            nodes[i].collided = false;
            if lost {
                collisions += 1;
                continue;
            }
            for j in 0..nodes.len() {
                if j == i || nodes[j].group != g || nodes[j].tx_end > 0.0 {
                    continue; // self, out-of-range, or deaf (transmitting)
                }
                // Pulse-coupled advance toward firing.
                nodes[j].phase = (nodes[j].phase + cfg.eps * (1.0 - nodes[j].phase)).min(1.0);
                deliveries += 1;
            }
        }

        // Sample the global order parameter.
        if step % 8 == 0 {
            let all: Vec<&Node> = nodes.iter().collect();
            let r = order_parameter(&all);
            r_series.push((t, r));
            if t >= late_start {
                late_r_sum += r;
                late_r_n += 1;
            }
        }
        t += DT;
    }

    let all: Vec<&Node> = nodes.iter().collect();
    let stats = Stats {
        final_r: order_parameter(&all),
        mean_late_r: if late_r_n > 0 { late_r_sum / late_r_n as f32 } else { 0.0 },
        collisions,
        deliveries,
    };
    (stats, r_series)
}

fn rng_frac(rng: &mut Lcg) -> f32 {
    rng.f01()
}
fn max_group(nodes: &[Node]) -> u8 {
    nodes.iter().map(|n| n.group).max().unwrap_or(0)
}

/// ASCII sparkline of R(t) (0 → ' ', 1 → '█').
fn spark(series: &[(f32, f32)]) -> String {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let mut s = String::new();
    // ~60 columns
    let stride = (series.len() / 60).max(1);
    for chunk in series.chunks(stride) {
        let r = chunk.iter().map(|(_, r)| r).sum::<f32>() / chunk.len() as f32;
        let idx = ((r * (RAMP.len() - 1) as f32).round() as usize).min(RAMP.len() - 1);
        s.push(RAMP[idx] as char);
    }
    s
}

fn main() {
    println!("Heartbeat-sync prototype — pulse-coupled oscillators over a modeled half-duplex LoRa channel");
    println!("(R = phase order parameter, 0=scattered .. 1=synchronised; sparkline is R over time)\n");

    let base = Config {
        n: 8,
        period: 1.5,
        eps: 0.18,
        jitter: 0.0,
        drift: 0.02,
        duration: 90.0,
    };

    // 1) IDEAL channel sanity — no radio constraints: must converge.
    //    (Modeled by zero airtime via a tiny period-independent delivery: here we
    //    approximate "ideal" as jittered-enough-to-never-collide with N=8.)
    let ideal = Config { jitter: 0.5, ..clone_cfg(&base) };
    let (s_ideal, r_ideal) = run(&ideal, 1, &[]);
    println!("1) ALGORITHM (well-spread TX, drift on): converges");
    println!("   R: [{}]  final={:.2} late_mean={:.2} collisions={} deliveries={}\n",
        spark(&r_ideal), s_ideal.final_r, s_ideal.mean_late_r, s_ideal.collisions, s_ideal.deliveries);

    // 2) NAIVE on real radio (jitter=0): synced firing → simultaneous TX → collide
    //    + deaf → drift wins. Claim: FAILS to hold sync.
    let naive = Config { jitter: 0.0, ..clone_cfg(&base) };
    let (s_naive, r_naive) = run(&naive, 1, &[]);
    println!("2) NAIVE (jitter=0) on half-duplex radio: simultaneous TX → collisions + deafness");
    println!("   R: [{}]  final={:.2} late_mean={:.2} collisions={} deliveries={}\n",
        spark(&r_naive), s_naive.final_r, s_naive.mean_late_r, s_naive.collisions, s_naive.deliveries);

    // 3) JITTER-DESYNC (the fix): spread TX in time → few collisions → sync holds.
    let jit = Config { jitter: 0.35, ..clone_cfg(&base) };
    let (s_jit, r_jit) = run(&jit, 1, &[]);
    println!("3) JITTER-DESYNC (jitter=0.35·T): LED phase syncs, radio announces spread");
    println!("   R: [{}]  final={:.2} late_mean={:.2} collisions={} deliveries={}\n",
        spark(&r_jit), s_jit.final_r, s_jit.mean_late_r, s_jit.collisions, s_jit.deliveries);

    // 4) PARTITION then HEAL — split into 2 groups at t=30, re-merge at t=60.
    let part = Config { jitter: 0.35, duration: 90.0, ..clone_cfg(&base) };
    let split: fn(usize) -> u8 = |i| (i % 2) as u8; // even/odd → 2 clusters
    let heal: fn(usize) -> u8 = |_| 0; // all back together
    let (s_part, r_part) = run(&part, 1, &[(30.0, split), (60.0, heal)]);
    println!("4) PARTITION (t=30, even/odd split) then HEAL (t=60): overall R dips, then re-syncs");
    println!("   R: [{}]  final={:.2}\n", spark(&r_part), s_part.final_r);

    // ── Assert the falsifiable claims ────────────────────────────────────────
    println!("── claims ──");
    let c1 = s_jit.mean_late_r > 0.9;
    let c2 = s_naive.mean_late_r < s_jit.mean_late_r - 0.2 && s_naive.collisions > s_jit.collisions * 3;
    println!("(1) jittered sync converges (late_mean R > 0.9):           {} ({:.2})", pass(c1), s_jit.mean_late_r);
    println!("(2) naive FAILS vs jittered (much lower R + many more collisions): {} (naive R {:.2} / {} coll vs jitter R {:.2} / {} coll)",
        pass(c2), s_naive.mean_late_r, s_naive.collisions, s_jit.mean_late_r, s_jit.collisions);
    let c3 = s_part.final_r > 0.85;
    println!("(3) partition→heal re-syncs (final R > 0.85):              {} ({:.2})", pass(c3), s_part.final_r);

    assert!(c1, "claim 1 (convergence) failed");
    assert!(c2, "claim 2 (naive fails on half-duplex) failed");
    assert!(c3, "claim 3 (heal re-syncs) failed");
    println!("\nAll claims hold. Tuned: eps={:.2}, jitter={:.2}·T, T={:.1}s, drift=±{:.0}%.",
        base.eps, 0.35, base.period, base.drift * 100.0);
}

fn clone_cfg(c: &Config) -> Config {
    Config { n: c.n, period: c.period, eps: c.eps, jitter: c.jitter, drift: c.drift, duration: c.duration }
}
fn pass(b: bool) -> &'static str {
    if b { "PASS" } else { "FAIL" }
}
