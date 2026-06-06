# agent-polyrhythm

> Polyrhythmic scheduling for heterogeneous agent fleets. Different agents, different rhythms, one groove.

In West African drumming, Cuban music, and jazz, **polyrhythm** is the art of playing two or more conflicting rhythms simultaneously. A 3-beat pattern against a 4-beat pattern. A 5-cycle over a 2-cycle. The magic happens in the *groove point* — the moment when all rhythms align — and in the syncopation that fills the space between alignments.

**agent-polyrhythm** applies this principle to agent fleet scheduling. Different agents naturally work at different rates: one processes tasks every 30 seconds, another every 45 seconds, a third every 2 minutes. Instead of forcing uniformity, this crate models those different rhythms as productive polyrhythms, detecting when cycles align and scheduling tasks accordingly.

## Core Concepts

### Beat

The fundamental unit of rhythmic time. Beats are `f64` values representing position in a rhythmic timeline.

```rust
use agent_polyrhythm::Beat;

let a = Beat(3.0);
let b = Beat(2.0);
let sum = a + b;  // Beat(5.0)
```

### RhythmicCycle

A repeating pattern with a fixed period. Think of it as one voice in the polyrhythm.

```rust
use agent_polyrhythm::RhythmicCycle;

let cycle = RhythmicCycle::new("health-check", 3.0)
    .with_phase(0.0)
    .with_velocity_pattern(vec![1.0, 0.5, 0.75]);

assert!(cycle.triggers_at(Beat(0.0)));   // triggers at 0
assert!(cycle.triggers_at(Beat(3.0)));   // triggers at 3
assert!(!cycle.triggers_at(Beat(1.0)));  // doesn't trigger at 1

// What's the next trigger after beat 1?
let next = cycle.next_trigger_after(Beat(1.0));  // Beat(3.0)

// Generate all triggers up to beat 12
let triggers = cycle.triggers_up_to(12.0);  // [0, 3, 6, 9, 12]
```

**Velocity patterns** control intensity within each cycle — like accent patterns in drumming:

```rust
let cycle = RhythmicCycle::new("pulse", 4.0)
    .with_velocity_pattern(vec![1.0, 0.5, 0.75, 0.5]);

let v = cycle.velocity_at(Beat(0.0));  // 1.0 (strong accent)
let v = cycle.velocity_at(Beat(1.0));  // 0.5 (lighter)
```

### Polyrhythm

Two or more cycles played simultaneously. The core abstraction.

```rust
use agent_polyrhythm::Polyrhythm;

// Classic 3:2 polyrhythm
let pr = Polyrhythm::simple("3-against-2", 2.0, 3.0);

assert_eq!(pr.voice_count(), 2);
assert!((pr.full_cycle_period() - 6.0).abs() < 1e-10);  // LCM(2,3) = 6

// Get all trigger events from all voices
let events = pr.all_triggers(12.0);
```

**Three or more voices:**

```rust
let pr = Polyrhythm::new("complex", vec![
    RhythmicCycle::new("fast", 2.0),
    RhythmicCycle::new("medium", 3.0),
    RhythmicCycle::new("slow", 5.0),
]);
// Full cycle: LCM(2,3,5) = 30 beats
```

### Groove Point

The moment when all cycles align — the "downbeat" of the polyrhythm.

```rust
use agent_polyrhythm::groove_point;

let cycles = vec![
    RhythmicCycle::new("a", 3.0),
    RhythmicCycle::new("b", 4.0),
];
let gp = groove_point(&cycles);  // Beat(12.0) — LCM(3,4)
```

For a 2:3:5 polyrhythm, the groove point is at beat 30 (LCM of 2, 3, and 5). Everything comes together.

### CycleAlignment

Detailed alignment detection — find every beat where 2+ cycles sync:

```rust
use agent_polyrhythm::{find_alignments, detect_alignments};

let cycles = vec![
    RhythmicCycle::new("a", 2.0),
    RhythmicCycle::new("b", 3.0),
];

// Simple: just the beat values
let alignments = find_alignments(&cycles, 12.0);  // [0, 6, 12]

// Detailed: which cycles align, is it full sync?
let details = detect_alignments(&cycles, 12.0);
for a in &details {
    println!("Beat {:.1}: {:?} sync={}", a.beat.0, a.aligned_cycles, a.is_full_sync);
}
// Beat 0.0: [0, 1] sync=true
// Beat 6.0: [0, 1] sync=true
// Beat 12.0: [0, 1] sync=true
```

### PolyrhythmicScheduler

Schedule tasks according to the polyrhythm. Each cycle can have assigned task templates:

```rust
use agent_polyrhythm::{Polyrhythm, PolyrhythmicScheduler};

let pr = Polyrhythm::simple("ops", 30.0, 45.0);  // 30s and 45s cycles
let mut scheduler = PolyrhythmicScheduler::new(pr);
scheduler.assign_task("voice-a", "check-health");
scheduler.assign_task("voice-b", "process-queue");

// Generate schedule for 180 seconds
let schedule = scheduler.generate_schedule(180.0);
for task in &schedule {
    println!("[{:.0}s] {} ({})", task.scheduled_beat.0, task.id, task.cycle_name);
}

// Or schedule exactly one full cycle
let one_cycle = scheduler.schedule_one_full_cycle();
```

Tasks carry priority (derived from velocity patterns) and can be marked complete:

```rust
let task = ScheduledTask::new("t1", "voice-a", Beat(30.0)).with_priority(0.9);
```

### SyncopationDetector

Identify off-beat patterns — beats that fall between the reference grid:

```rust
use agent_polyrhythm::SyncopationDetector;

let detector = SyncopationDetector::new(1.0);  // reference: every 1 beat

let beats = vec![Beat(0.0), Beat(0.5), Beat(1.0), Beat(1.5)];
let events = detector.detect(&beats);
// Beat 0.0: on-grid
// Beat 0.5: off-grid (syncopated!)
// Beat 1.0: on-grid
// Beat 1.5: off-grid (syncopated!)

let ratio = detector.syncopation_ratio(&beats);  // 0.5
assert!(!detector.is_highly_syncopated(&beats));
```

**Syncopation strength** measures how "off" a beat is — a beat at 0.5 in a period of 1.0 is maximally syncopated, while 0.1 is barely off-grid.

## Design Philosophy

This crate treats rhythm as a first-class scheduling primitive. In music, polyrhythm isn't chaos — it's structured tension and release. The same applies to agent fleets:

- **Different natural rhythms** are a feature, not a bug
- **Groove points** are moments of synchronization — opportunities for coordination
- **Syncopation** creates productive variety — not everything has to happen on the beat
- **Cycle alignment** tells you when agents will naturally converge

The musical metaphor extends to velocity patterns (accent structures), phase offsets (starting points), and the idea that the space between alignments is where interesting things happen.

## Running Tests

```bash
cargo test
```

All 23 tests cover: beats, cycle creation and triggers, phase offsets, polyrhythm construction, groove point detection, alignment finding, scheduling, and syncopation detection.

## License

MIT
