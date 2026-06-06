//! # agent-polyrhythm
//!
//! Polyrhythmic scheduling for heterogeneous agent fleets. Different agents
//! naturally work at different rhythms — this crate models conflicting rhythms
//! as productive polyrhythms, schedules tasks at different rates, detects groove
//! points where cycles align, and identifies syncopation patterns.

use std::collections::{HashMap, BTreeMap};
use std::ops::{Add, Sub};

/// A moment in rhythmic time, measured in beats.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Beat(pub f64);

impl Beat {
    pub fn zero() -> Self { Beat(0.0) }
    pub fn is_zero(&self) -> bool { self.0.abs() < f64::EPSILON }
}

impl Add for Beat {
    type Output = Beat;
    fn add(self, rhs: Self) -> Self::Output { Beat(self.0 + rhs.0) }
}

impl Sub for Beat {
    type Output = Beat;
    fn sub(self, rhs: Self) -> Self::Output { Beat(self.0 - rhs.0) }
}

/// A repeating rhythmic pattern with a fixed period.
#[derive(Debug, Clone)]
pub struct RhythmicCycle {
    /// Name/identifier for this cycle.
    pub name: String,
    /// Period in beats (e.g., 3.0 = every 3 beats).
    pub period: Beat,
    /// Phase offset in beats (starts shifted).
    pub phase: Beat,
    /// Optional velocity/intensity pattern within one cycle (values in [0.0, 1.0]).
    pub velocity_pattern: Vec<f64>,
}

impl RhythmicCycle {
    /// Create a new rhythmic cycle with the given period.
    pub fn new(name: impl Into<String>, period: f64) -> Self {
        Self {
            name: name.into(),
            period: Beat(period),
            phase: Beat::zero(),
            velocity_pattern: Vec::new(),
        }
    }

    /// Set phase offset.
    pub fn with_phase(mut self, phase: f64) -> Self {
        self.phase = Beat(phase);
        self
    }

    /// Set velocity pattern (intensities within one cycle).
    pub fn with_velocity_pattern(mut self, pattern: Vec<f64>) -> Self {
        self.velocity_pattern = pattern;
        self
    }

    /// Does this cycle trigger at the given beat?
    pub fn triggers_at(&self, beat: Beat) -> bool {
        let adjusted = beat.0 - self.phase.0;
        if adjusted < 0.0 { return false; }
        (adjusted % self.period.0).abs() < 1e-10
    }

    /// Next trigger after the given beat (exclusive).
    pub fn next_trigger_after(&self, beat: Beat) -> Beat {
        let adjusted = beat.0 - self.phase.0;
        if adjusted < 0.0 {
            return self.phase;
        }
        let remainder = adjusted % self.period.0;
        let next = if remainder.abs() < 1e-10 {
            beat.0 + self.period.0
        } else {
            beat.0 + (self.period.0 - remainder)
        };
        Beat(next)
    }

    /// Generate trigger beats up to a given limit.
    pub fn triggers_up_to(&self, max_beat: f64) -> Vec<Beat> {
        let mut triggers = Vec::new();
        let mut current = self.phase.0;
        if current < 0.0 { current = 0.0; }
        while current <= max_beat + 1e-10 {
            triggers.push(Beat(current));
            current += self.period.0;
        }
        triggers
    }

    /// Velocity at a given beat (based on velocity_pattern).
    pub fn velocity_at(&self, beat: Beat) -> f64 {
        if self.velocity_pattern.is_empty() { return 1.0; }
        let adjusted = beat.0 - self.phase.0;
        if adjusted < 0.0 { return 0.0; }
        let cycle_position = adjusted % self.period.0;
        let step_size = self.period.0 / self.velocity_pattern.len() as f64;
        let idx = (cycle_position / step_size).floor() as usize;
        self.velocity_pattern.get(idx % self.velocity_pattern.len()).copied().unwrap_or(1.0)
    }

    /// How many triggers occur in [0, duration]?
    pub fn count_triggers(&self, duration: f64) -> usize {
        self.triggers_up_to(duration).len()
    }
}

/// A polyrhythm: two or more conflicting rhythms played simultaneously.
#[derive(Debug, Clone)]
pub struct Polyrhythm {
    /// The cycles that make up this polyrhythm.
    pub cycles: Vec<RhythmicCycle>,
    /// Name for this polyrhythm pattern.
    pub name: String,
}

impl Polyrhythm {
    /// Create a polyrhythm from cycles.
    pub fn new(name: impl Into<String>, cycles: Vec<RhythmicCycle>) -> Self {
        Self { cycles, name: name.into() }
    }

    /// Create a simple 2-voice polyrhythm (e.g., 3:2, 4:3).
    pub fn simple(name: impl Into<String>, period_a: f64, period_b: f64) -> Self {
        let a = RhythmicCycle::new("voice-a", period_a);
        let b = RhythmicCycle::new("voice-b", period_b);
        Self { cycles: vec![a, b], name: name.into() }
    }

    /// All trigger points from all cycles up to max_beat.
    pub fn all_triggers(&self, max_beat: f64) -> Vec<(usize, Beat)> {
        let mut triggers = Vec::new();
        for (i, cycle) in self.cycles.iter().enumerate() {
            for beat in cycle.triggers_up_to(max_beat) {
                triggers.push((i, beat));
            }
        }
        triggers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        triggers
    }

    /// Number of cycles/voices.
    pub fn voice_count(&self) -> usize {
        self.cycles.len()
    }

    /// The least common multiple of all cycle periods (the full pattern period).
    /// Works for integer-valued periods.
    pub fn full_cycle_period(&self) -> f64 {
        let periods: Vec<u64> = self.cycles.iter()
            .map(|c| c.period.0.round() as u64)
            .filter(|&p| p > 0)
            .collect();
        if periods.is_empty() { return 0.0; }
        periods.iter().fold(1u64, |acc, &p| lcm(acc, p)) as f64
    }
}

/// Find the groove point: the earliest beat where all cycles align.
/// For integer periods, this is the LCM of the periods.
pub fn groove_point(cycles: &[RhythmicCycle]) -> Beat {
    let periods: Vec<u64> = cycles.iter()
        .map(|c| c.period.0.round() as u64)
        .filter(|&p| p > 0)
        .collect();
    if periods.is_empty() { return Beat::zero(); }
    let lcm_val = periods.iter().fold(1u64, |acc, &p| lcm(acc, p));
    Beat(lcm_val as f64)
}

/// Find all alignment points (beats where 2+ cycles trigger simultaneously).
pub fn find_alignments(cycles: &[RhythmicCycle], max_beat: f64) -> Vec<Beat> {
    let mut beat_counts: BTreeMap<u64, usize> = BTreeMap::new();
    for cycle in cycles {
        for beat in cycle.triggers_up_to(max_beat) {
            let key = (beat.0 * 1e6).round() as u64;
            *beat_counts.entry(key).or_insert(0) += 1;
        }
    }
    beat_counts.into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(key, _)| Beat(key as f64 / 1e6))
        .collect()
}

/// Cycle alignment info: details about when cycles sync.
#[derive(Debug, Clone)]
pub struct CycleAlignment {
    /// The beat at which alignment occurs.
    pub beat: Beat,
    /// Which cycle indices are aligned at this point.
    pub aligned_cycles: Vec<usize>,
    /// Is this the "groove point" (all cycles aligned)?
    pub is_full_sync: bool,
}

/// Detect all cycle alignments with details.
pub fn detect_alignments(cycles: &[RhythmicCycle], max_beat: f64) -> Vec<CycleAlignment> {
    let mut beat_to_cycles: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (i, cycle) in cycles.iter().enumerate() {
        for beat in cycle.triggers_up_to(max_beat) {
            let key = (beat.0 * 1e6).round() as u64;
            beat_to_cycles.entry(key).or_default().push(i);
        }
    }
    beat_to_cycles.into_iter()
        .filter(|(_, v)| v.len() >= 2)
        .map(|(key, aligned_cycles)| {
            let beat_val = key as f64 / 1e6;
            CycleAlignment {
                beat: Beat(beat_val),
                is_full_sync: aligned_cycles.len() == cycles.len(),
                aligned_cycles,
            }
        })
        .collect()
}

/// A scheduled task in the polyrhythmic framework.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task identifier.
    pub id: String,
    /// The cycle this task runs on.
    pub cycle_name: String,
    /// What beat this task is scheduled at.
    pub scheduled_beat: Beat,
    /// Task priority (higher = more important).
    pub priority: f64,
    /// Was this task completed?
    pub completed: bool,
}

impl ScheduledTask {
    pub fn new(id: impl Into<String>, cycle_name: impl Into<String>, beat: Beat) -> Self {
        Self {
            id: id.into(),
            cycle_name: cycle_name.into(),
            scheduled_beat: beat,
            priority: 1.0,
            completed: false,
        }
    }

    pub fn with_priority(mut self, p: f64) -> Self {
        self.priority = p;
        self
    }
}

/// A polyrhythmic scheduler: assigns tasks to cycles and generates schedules.
#[derive(Debug, Clone)]
pub struct PolyrhythmicScheduler {
    /// The polyrhythm definition.
    pub polyrhythm: Polyrhythm,
    /// Tasks scheduled per cycle, keyed by cycle name.
    pub task_templates: HashMap<String, Vec<String>>,
}

impl PolyrhythmicScheduler {
    /// Create a scheduler for a given polyrhythm.
    pub fn new(polyrhythm: Polyrhythm) -> Self {
        Self {
            polyrhythm,
            task_templates: HashMap::new(),
        }
    }

    /// Assign a task template to a cycle (task name is generated per trigger).
    pub fn assign_task(&mut self, cycle_name: &str, task_name: &str) {
        self.task_templates.entry(cycle_name.to_string())
            .or_default().push(task_name.to_string());
    }

    /// Generate the full schedule up to max_beat.
    pub fn generate_schedule(&self, max_beat: f64) -> Vec<ScheduledTask> {
        let mut tasks = Vec::new();
        let mut task_counter = 0u64;
        for (cycle_idx, cycle) in self.polyrhythm.cycles.iter().enumerate() {
            if let Some(template_names) = self.task_templates.get(&cycle.name) {
                for beat in cycle.triggers_up_to(max_beat) {
                    for tmpl in template_names {
                        task_counter += 1;
                        let velocity = cycle.velocity_at(beat);
                        tasks.push(ScheduledTask::new(
                            format!("{}-{}", tmpl, task_counter),
                            &cycle.name,
                            beat,
                        ).with_priority(velocity));
                    }
                }
            } else {
                // Default: one task per trigger
                for beat in cycle.triggers_up_to(max_beat) {
                    task_counter += 1;
                    tasks.push(ScheduledTask::new(
                        format!("task-{}", task_counter),
                        &cycle.name,
                        beat,
                    ));
                }
            }
        }
        tasks.sort_by(|a, b| a.scheduled_beat.partial_cmp(&b.scheduled_beat).unwrap());
        tasks
    }

    /// Tasks scheduled at a specific beat.
    pub fn tasks_at(&self, beat: Beat, max_beat: f64) -> Vec<ScheduledTask> {
        let schedule = self.generate_schedule(max_beat);
        schedule.into_iter().filter(|t| (t.scheduled_beat.0 - beat.0).abs() < 1e-10).collect()
    }

    /// Schedule up to the next groove point (one full cycle).
    pub fn schedule_one_full_cycle(&self) -> Vec<ScheduledTask> {
        let period = self.polyrhythm.full_cycle_period();
        if period == 0.0 { return Vec::new(); }
        self.generate_schedule(period)
    }
}

/// Detect syncopation: off-beat patterns in a sequence of beats relative to a reference pulse.
#[derive(Debug, Clone)]
pub struct SyncopationDetector {
    /// The reference pulse period (e.g., 1.0 for quarter notes).
    pub reference_period: f64,
}

impl SyncopationDetector {
    pub fn new(reference_period: f64) -> Self {
        Self { reference_period }
    }

    /// Detect syncopated beats: beats that fall off the reference grid.
    pub fn detect(&self, beats: &[Beat]) -> Vec<SyncopationEvent> {
        beats.iter().enumerate().map(|(i, &beat)| {
            let on_grid = (beat.0 % self.reference_period).abs() < 1e-10;
            let offset = beat.0 % self.reference_period;
            SyncopationEvent {
                beat,
                on_grid,
                offset_from_grid: if offset.abs() < 1e-10 { 0.0 } else { offset },
                syncopation_strength: if on_grid { 0.0 } else { 1.0 - (offset / self.reference_period - 0.5).abs() * 2.0 },
            }
        }).collect()
    }

    /// Overall syncopation ratio: fraction of beats that are off-grid.
    pub fn syncopation_ratio(&self, beats: &[Beat]) -> f64 {
        if beats.is_empty() { return 0.0; }
        let events = self.detect(beats);
        events.iter().filter(|e| !e.on_grid).count() as f64 / beats.len() as f64
    }

    /// Are these beats highly syncopated (> 50% off-grid)?
    pub fn is_highly_syncopated(&self, beats: &[Beat]) -> bool {
        self.syncopation_ratio(beats) > 0.5
    }
}

/// A syncopation event: a beat classified as on-grid or off-grid.
#[derive(Debug, Clone, Copy)]
pub struct SyncopationEvent {
    /// The beat.
    pub beat: Beat,
    /// Is it on the reference grid?
    pub on_grid: bool,
    /// Offset from the nearest grid point.
    pub offset_from_grid: f64,
    /// How "syncopated" this is (0.0 = on grid, 1.0 = maximally off-beat).
    pub syncopation_strength: f64,
}

/// Compute GCD of two numbers.
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Compute LCM of two numbers.
fn lcm(a: u64, b: u64) -> u64 {
    a / gcd(a, b) * b
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_arithmetic() {
        let a = Beat(3.0);
        let b = Beat(2.0);
        assert_eq!((a + b).0, 5.0);
        assert_eq!((a - b).0, 1.0);
    }

    #[test]
    fn test_rhythmic_cycle_creation() {
        let cycle = RhythmicCycle::new("pulse", 4.0);
        assert_eq!(cycle.period.0, 4.0);
        assert_eq!(cycle.phase.0, 0.0);
    }

    #[test]
    fn test_rhythmic_cycle_triggers() {
        let cycle = RhythmicCycle::new("pulse", 3.0);
        assert!(cycle.triggers_at(Beat(0.0)));
        assert!(cycle.triggers_at(Beat(3.0)));
        assert!(cycle.triggers_at(Beat(6.0)));
        assert!(!cycle.triggers_at(Beat(1.0)));
        assert!(!cycle.triggers_at(Beat(2.5)));
    }

    #[test]
    fn test_rhythmic_cycle_with_phase() {
        let cycle = RhythmicCycle::new("offset", 2.0).with_phase(1.0);
        assert!(!cycle.triggers_at(Beat(0.0)));
        assert!(cycle.triggers_at(Beat(1.0)));
        assert!(cycle.triggers_at(Beat(3.0)));
    }

    #[test]
    fn test_rhythmic_cycle_next_trigger() {
        let cycle = RhythmicCycle::new("pulse", 4.0);
        let next = cycle.next_trigger_after(Beat(1.0));
        assert!((next.0 - 4.0).abs() < 1e-10);
        let next2 = cycle.next_trigger_after(Beat(4.0));
        assert!((next2.0 - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_rhythmic_cycle_triggers_up_to() {
        let cycle = RhythmicCycle::new("pulse", 2.0);
        let triggers = cycle.triggers_up_to(8.0);
        assert_eq!(triggers.len(), 5); // 0, 2, 4, 6, 8
    }

    #[test]
    fn test_rhythmic_cycle_count() {
        let cycle = RhythmicCycle::new("pulse", 3.0);
        assert_eq!(cycle.count_triggers(12.0), 5); // 0, 3, 6, 9, 12
    }

    #[test]
    fn test_velocity_pattern() {
        let cycle = RhythmicCycle::new("accent", 4.0)
            .with_velocity_pattern(vec![1.0, 0.5, 0.75, 0.5]);
        assert!((cycle.velocity_at(Beat(0.0)) - 1.0).abs() < 1e-10);
        assert!((cycle.velocity_at(Beat(1.0)) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_polyrhythm_simple() {
        let pr = Polyrhythm::simple("3:2", 2.0, 3.0);
        assert_eq!(pr.voice_count(), 2);
        let period = pr.full_cycle_period();
        assert!((period - 6.0).abs() < 1e-10); // LCM(2,3) = 6
    }

    #[test]
    fn test_polyrhythm_triggers() {
        let pr = Polyrhythm::simple("3:2", 2.0, 3.0);
        let triggers = pr.all_triggers(6.0);
        assert!(triggers.len() >= 6); // 0,2,4,6 from cycle A + 0,3,6 from cycle B
    }

    #[test]
    fn test_polyrhythm_full_cycle_4_3() {
        let pr = Polyrhythm::simple("4:3", 3.0, 4.0);
        assert!((pr.full_cycle_period() - 12.0).abs() < 1e-10); // LCM(3,4) = 12
    }

    #[test]
    fn test_groove_point() {
        let cycles = vec![
            RhythmicCycle::new("a", 3.0),
            RhythmicCycle::new("b", 4.0),
        ];
        let gp = groove_point(&cycles);
        assert!((gp.0 - 12.0).abs() < 1e-10); // LCM(3,4) = 12
    }

    #[test]
    fn test_groove_point_three_way() {
        let cycles = vec![
            RhythmicCycle::new("a", 2.0),
            RhythmicCycle::new("b", 3.0),
            RhythmicCycle::new("c", 5.0),
        ];
        let gp = groove_point(&cycles);
        assert!((gp.0 - 30.0).abs() < 1e-10); // LCM(2,3,5) = 30
    }

    #[test]
    fn test_find_alignments() {
        let cycles = vec![
            RhythmicCycle::new("a", 2.0),
            RhythmicCycle::new("b", 3.0),
        ];
        let alignments = find_alignments(&cycles, 12.0);
        assert!(alignments.contains(&Beat(0.0)));
        assert!(alignments.contains(&Beat(6.0)));
        assert!(alignments.contains(&Beat(12.0)));
        assert!(!alignments.contains(&Beat(2.0)));
    }

    #[test]
    fn test_detect_alignments_detailed() {
        let cycles = vec![
            RhythmicCycle::new("a", 2.0),
            RhythmicCycle::new("b", 3.0),
        ];
        let alignments = detect_alignments(&cycles, 12.0);
        let six = alignments.iter().find(|a| (a.beat.0 - 6.0).abs() < 1e-10).unwrap();
        assert!(six.is_full_sync);
        assert_eq!(six.aligned_cycles.len(), 2);
    }

    #[test]
    fn test_scheduler_basic() {
        let pr = Polyrhythm::simple("3:2", 3.0, 4.0);
        let mut scheduler = PolyrhythmicScheduler::new(pr);
        scheduler.assign_task("voice-a", "check-health");
        scheduler.assign_task("voice-b", "process-queue");
        let schedule = scheduler.generate_schedule(12.0);
        assert!(!schedule.is_empty());
        // Should be sorted by beat
        for i in 1..schedule.len() {
            assert!(schedule[i].scheduled_beat.0 >= schedule[i-1].scheduled_beat.0);
        }
    }

    #[test]
    fn test_scheduler_one_full_cycle() {
        let pr = Polyrhythm::simple("3:2", 3.0, 4.0);
        let scheduler = PolyrhythmicScheduler::new(pr);
        let schedule = scheduler.schedule_one_full_cycle();
        assert!(!schedule.is_empty());
    }

    #[test]
    fn test_syncopation_detector_on_grid() {
        let detector = SyncopationDetector::new(1.0);
        let beats = vec![Beat(0.0), Beat(1.0), Beat(2.0), Beat(3.0)];
        let events = detector.detect(&beats);
        assert!(events.iter().all(|e| e.on_grid));
        assert!((detector.syncopation_ratio(&beats) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_syncopation_detector_off_grid() {
        let detector = SyncopationDetector::new(1.0);
        let beats = vec![Beat(0.5), Beat(1.5), Beat(2.5)];
        let events = detector.detect(&beats);
        assert!(events.iter().all(|e| !e.on_grid));
        assert!((detector.syncopation_ratio(&beats) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_syncopation_mixed() {
        let detector = SyncopationDetector::new(1.0);
        let beats = vec![Beat(0.0), Beat(0.5), Beat(1.0), Beat(1.5)];
        let ratio = detector.syncopation_ratio(&beats);
        assert!((ratio - 0.5).abs() < 1e-10);
        assert!(!detector.is_highly_syncopated(&beats));
    }

    #[test]
    fn test_syncopation_highly() {
        let detector = SyncopationDetector::new(4.0);
        let beats = vec![Beat(1.0), Beat(3.0), Beat(5.0), Beat(7.0)];
        // All off-grid relative to period 4
        assert!(detector.is_highly_syncopated(&beats));
    }

    #[test]
    fn test_scheduled_task_creation() {
        let task = ScheduledTask::new("t1", "voice-a", Beat(3.0)).with_priority(0.8);
        assert_eq!(task.id, "t1");
        assert!((task.priority - 0.8).abs() < 1e-10);
        assert!(!task.completed);
    }

    #[test]
    fn test_polyrhythm_three_voices() {
        let pr = Polyrhythm::new("complex", vec![
            RhythmicCycle::new("a", 2.0),
            RhythmicCycle::new("b", 3.0),
            RhythmicCycle::new("c", 5.0),
        ]);
        assert_eq!(pr.voice_count(), 3);
        assert!((pr.full_cycle_period() - 30.0).abs() < 1e-10);
    }
}
