use agent_polyrhythm::*;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║             🥁 Polyrhythm Grid Visualization 🥁             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // === 3:2 Polyrhythm ===
    println!("━━━ Polyrhythm 3:2 (e.g., 3 beats against 2 beats) ━━━");
    println!();

    let three_two = Polyrhythm::simple("3:2", 2.0, 3.0);
    println!("  Voice A period: 2.0 beats (plays 3 times in 6 beats)");
    println!("  Voice B period: 3.0 beats (plays 2 times in 6 beats)");
    println!("  Full cycle:     {:.0} beats (LCM of 2,3)", three_two.full_cycle_period());
    println!();

    print_rhythmic_grid(&three_two, 6.0);
    println!();

    // Alignment points
    let cycles_32 = &three_two.cycles;
    let alignments = detect_alignments(cycles_32, 6.0);
    println!("  Alignment points (both voices hit):");
    for a in &alignments {
        println!("    Beat {:.0} ← {} FULL SYNC {}", a.beat.0,
            if a.is_full_sync { "⭐" } else { "" },
            if a.is_full_sync { "⭐" } else { "" });
    }
    println!();

    // === 4:3 Polyrhythm ===
    println!("━━━ Polyrhythm 4:3 (e.g., 4 beats against 3 beats) ━━━");
    println!();

    let four_three = Polyrhythm::simple("4:3", 3.0, 4.0);
    println!("  Voice A period: 3.0 beats (plays 4 times in 12 beats)");
    println!("  Voice B period: 4.0 beats (plays 3 times in 12 beats)");
    println!("  Full cycle:     {:.0} beats (LCM of 3,4)", four_three.full_cycle_period());
    println!();

    print_rhythmic_grid(&four_three, 12.0);
    println!();

    let alignments_43 = detect_alignments(&four_three.cycles, 12.0);
    println!("  Alignment points:");
    for a in &alignments_43 {
        println!("    Beat {:.0} {}", a.beat.0,
            if a.is_full_sync { "⭐ FULL SYNC" } else { "" });
    }
    println!();

    // === Three-way Polyrhythm: 2:3:5 ===
    println!("━━━ Polyrhythm 2:3:5 (three voices!) ━━━");
    println!();

    let complex = Polyrhythm::new("2:3:5", vec![
        RhythmicCycle::new("fast", 2.0),
        RhythmicCycle::new("medium", 3.0),
        RhythmicCycle::new("slow", 5.0),
    ]);
    println!("  Fast:   period 2  ({} hits in {} beats)", complex.cycles[0].count_triggers(30.0) - 1, 30);
    println!("  Medium: period 3  ({} hits in {} beats)", complex.cycles[1].count_triggers(30.0) - 1, 30);
    println!("  Slow:   period 5  ({} hits in {} beats)", complex.cycles[2].count_triggers(30.0) - 1, 30);
    println!("  Full cycle: {:.0} beats (LCM of 2,3,5)", complex.full_cycle_period());
    println!();

    print_rhythmic_grid(&complex, 30.0);
    println!();

    // === Syncopation Detection ===
    println!("━━━ Syncopation Detection ━━━");
    println!();

    let detector = SyncopationDetector::new(1.0);

    let straight = vec![Beat(0.0), Beat(1.0), Beat(2.0), Beat(3.0), Beat(4.0)];
    let syncopated = vec![Beat(0.5), Beat(1.5), Beat(2.5), Beat(3.5)];
    let mixed = vec![Beat(0.0), Beat(0.5), Beat(1.0), Beat(1.5), Beat(2.0), Beat(2.75)];

    println!("  Straight beats:    syncopation = {:.0}%  {}",
        detector.syncopation_ratio(&straight) * 100.0,
        if detector.is_highly_syncopated(&straight) { "⚠️ HIGH" } else { "✅ low" });
    println!("  Syncopated beats:  syncopation = {:.0}%  {}",
        detector.syncopation_ratio(&syncopated) * 100.0,
        if detector.is_highly_syncopated(&syncopated) { "⚠️ HIGH" } else { "✅ low" });
    println!("  Mixed beats:       syncopation = {:.0}%  {}",
        detector.syncopation_ratio(&mixed) * 100.0,
        if detector.is_highly_syncopated(&mixed) { "⚠️ HIGH" } else { "✅ low" });
    println!();

    // Detailed syncopation events
    println!("  Detailed syncopation analysis of mixed pattern:");
    for event in detector.detect(&mixed) {
        let status = if event.on_grid { "│" } else { "╳" };
        println!("    Beat {:5.2}  {}  offset={:.2}  strength={:.2}",
            event.beat.0, status, event.offset_from_grid, event.syncopation_strength);
    }
    println!();

    // === Scheduler Demo ===
    println!("━━━ Polyrhythmic Scheduler ━━━");
    println!();

    let pr = Polyrhythm::simple("task-rhythm", 3.0, 4.0);
    let mut scheduler = PolyrhythmicScheduler::new(pr);
    scheduler.assign_task("voice-a", "health-check");
    scheduler.assign_task("voice-b", "process-queue");

    let schedule = scheduler.schedule_one_full_cycle();
    println!("  Schedule for one full 4:3 cycle (12 beats):");
    println!("  {:>4}  {:>12}  {:>8}  {:>10}", "Beat", "Task ID", "Cycle", "Priority");
    println!("  {}  {}  {}  {}", "────", "────────────", "──────", "──────────");
    for task in &schedule {
        println!("  {:>4.0}  {:>12}  {:>8}  {:>10.2}",
            task.scheduled_beat.0, task.id, task.cycle_name, task.priority);
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     Where rhythms collide, music happens 🎵                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

fn print_rhythmic_grid(polyrhythm: &Polyrhythm, max_beat: f64) {
    let voice_count = polyrhythm.voice_count();
    let period = polyrhythm.full_cycle_period();
    let display_max = if max_beat <= 16.0 { max_beat } else { period };
    let steps = display_max as usize;

    // Collect triggers per voice
    let mut voice_triggers: Vec<std::collections::HashSet<usize>> = vec![std::collections::HashSet::new(); voice_count];
    for (vi, beat) in polyrhythm.all_triggers(display_max) {
        voice_triggers[vi].insert(beat.0.round() as usize);
    }

    // Print grid
    let labels: Vec<&str> = polyrhythm.cycles.iter().map(|c| c.name.as_str()).collect();
    let max_label_len = labels.iter().map(|l| l.len()).max().unwrap_or(8).max(8);

    // Header with beat numbers
    print!("  {:>width$} │", "beat:", width = max_label_len);
    for b in 0..=steps {
        print!("{}", if b % 5 == 0 { format!("{:<2}", b % 10) } else { "  ".to_string() });
    }
    println!();
    print!("  {:>width$} │", "", width = max_label_len);
    for _ in 0..=steps { print!("──"); }
    println!();

    for vi in 0..voice_count {
        print!("  {:>width$} │", labels[vi], width = max_label_len);
        for b in 0..=steps {
            if voice_triggers[vi].contains(&b) {
                // Check if other voices also trigger here
                let count = voice_triggers.iter().filter(|t| t.contains(&b)).count();
                if count >= 2 {
                    print!("⭐"); // alignment!
                } else {
                    print!("● ");
                }
            } else {
                print!("· ");
            }
        }
        println!();
    }
}
