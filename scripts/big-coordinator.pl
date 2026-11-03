#!/usr/bin/env perl
use strict;
use warnings;
use Math::BigInt try => 'GMP';
use JSON::PP qw(encode_json decode_json);
use POSIX qw(:sys_wait_h);
use FindBin;
use Fcntl qw(:flock);
use Time::HiRes qw(time);
use File::Path qw(make_path);

my ($bin, $root, $u128_root, $threads, $test_max_bits) = @ARGV;
die "usage: big-coordinator.pl BIN BIG_ROOT U128_ROOT THREADS\n" unless $threads;
$test_max_bits =~ s/^--max-bits=// if defined $test_max_bits;
die "invalid test bit limit\n" if defined($test_max_bits) && $test_max_bits !~ /\A(?:[2-9]|[1-9][0-9]|1[0-9]{2}|2[0-5][0-6])\z/;
$threads = int($threads);
die "threads must be positive\n" unless $threads > 0;
make_path($root);
open my $lock, '>', "$root/coordinator.lock" or die $!;
flock($lock, LOCK_EX | LOCK_NB) or die "another BigInt coordinator holds the lock\n";

my %counts;
open my $space, '-|', $bin, 'space256' or die "cannot run space256: $!";
while (<$space>) {
    $counts{$1} = Math::BigInt->new($2) if /^bits=(\d+).*assignments=(\d+)/;
}
close $space or die "space256 failed\n";
my $total = Math::BigInt->bzero;
my $through128 = Math::BigInt->bzero;
for my $b (2..256) { die "missing count for $b\n" unless $counts{$b}; $total->badd($counts{$b}); $through128->badd($counts{$b}) if $b <= 128; }

my $state_path = "$root/checkpoint.json";
my $state = -e $state_path ? decode_json(do { local $/; open my $f, '<', $state_path or die $!; <$f> }) : {
    status => 'running', phase => 'common_2_128', bits => 2, next_start => '0',
    completed_assignments => '0', partitions => 0, result_records => 0,
    prime_records => 0, composite_records => 0, unresolved_records => 0,
    active_seconds => 0, started_epoch => time(), started_utc => utc(),
};
$state->{started_epoch} //= time();
$state->{started_utc} //= utc();

sub utc { my @t = gmtime(); return sprintf('%04d-%02d-%02dT%02d:%02d:%02dZ', $t[5]+1900,$t[4]+1,$t[3],$t[2],$t[1],$t[0]); }
sub atomic_json {
    my ($path, $obj) = @_;
    my $tmp = "$path.tmp.$$";
    open my $f, '>', $tmp or die "open $tmp: $!";
    print {$f} encode_json($obj), "\n" or die "write $tmp: $!";
    $f->flush(); $f->sync() or die "sync $tmp: $!";
    close $f or die "close $tmp: $!";
    rename $tmp, $path or die "rename $tmp: $!";
    open my $d, '<', "$root" or die "open directory: $!";
    $d->sync() or die "sync directory: $!";
    close $d;
}
sub atomic_text {
    my ($path, $text) = @_;
    my $tmp = "$path.tmp.$$";
    open my $f, '>', $tmp or die "open $tmp: $!";
    print {$f} $text or die "write $tmp: $!";
    $f->flush(); $f->sync() or die "sync $tmp: $!";
    close $f or die "close $tmp: $!";
    rename $tmp, $path or die "rename $tmp: $!";
    open my $d, '<', "$root" or die "open directory: $!";
    $d->sync() or die "sync directory: $!";
    close $d;
}
sub save {
    my ($event) = @_;
    $state->{updated_utc} = utc();
    $state->{total_assignments} = "$total";
    $state->{through_128_assignments} = "$through128";
    $state->{event} = $event;
    my $done = Math::BigInt->new($state->{completed_assignments});
    my $pct = $total->is_zero ? 100 : (100 * $done->numify() / $total->numify());
    $state->{percentage_complete} = sprintf('%.12f', $pct);
    my $common_done = $done->copy();
    $common_done = $through128->copy() if $common_done->bcmp($through128) > 0;
    $state->{percentage_of_2_128_workload} = sprintf('%.9f', 100 * $common_done->numify() / $through128->numify());
    my $elapsed = time() - $state->{started_epoch};
    $state->{elapsed_wall_seconds} = sprintf('%.3f', $elapsed);
    $state->{active_seconds} = sprintf('%.3f', ($state->{active_seconds} || 0) + ($state->{batch_active_seconds} || 0));
    delete $state->{batch_active_seconds};
    atomic_json($state_path, $state);
    atomic_json("$root/final-report.json", {
        status => $state->{status}, phase => $state->{phase}, current_bit_length => $state->{bits},
        next_assignment => $state->{next_start}, completed_assignments => $state->{completed_assignments},
        total_assignments => "$total", through_128_assignments => "$through128",
        percentage_complete => 0 + $state->{percentage_complete}, completed_partitions => $state->{partitions},
        percentage_of_2_128_workload => 0 + $state->{percentage_of_2_128_workload},
        triple_palindrome_records => $state->{result_records}, prime_classifications => $state->{prime_records},
        composite_classifications => $state->{composite_records}, unresolved_classifications => $state->{unresolved_records},
        active_seconds => 0 + $state->{active_seconds}, started_utc => $state->{started_utc},
        elapsed_wall_seconds => 0 + $state->{elapsed_wall_seconds},
        average_assignments_per_active_second => $state->{active_seconds} > 0
            ? 0 + sprintf('%.3f', Math::BigInt->new($state->{completed_assignments})->numify() / $state->{active_seconds})
            : 0,
        updated_utc => $state->{updated_utc}, last_event => $event,
        note => 'Incremental BigUint status. Primality records marked unresolved are not proven primes.'
    });
    my $comparison = $state->{phase} eq 'common_2_128' ? 'pending (the coordinator will compare before starting bit length 129)' :
        ($state->{phase} eq 'compare_2_128' ? 'running' : 'passed');
    my $human = "# BigUint exhaustive search progress\n\n" .
        "- **Status:** $state->{status}\n" .
        "- **Phase:** $state->{phase}\n" .
        "- **2–128-bit cross-check:** $comparison\n" .
        "- **Current bit length:** $state->{bits}\n" .
        "- **Next assignment index:** $state->{next_start}\n" .
        "- **Completed candidate assignments:** $state->{completed_assignments} / $total\n" .
        "- **Overall workload complete:** $state->{percentage_complete}%\n" .
        "- **2–128-bit workload complete:** $state->{percentage_of_2_128_workload}%\n" .
        "- **Completed partitions:** $state->{partitions}\n" .
        "- **Triple-palindrome records:** $state->{result_records}\n" .
        "- **Prime classifications:** $state->{prime_records}\n" .
        "- **Composite classifications:** $state->{composite_records}\n" .
        "- **Unresolved classifications:** $state->{unresolved_records}\n" .
        "- **Active worker batch time:** $state->{active_seconds} seconds\n" .
        "- **Elapsed wall time:** $state->{elapsed_wall_seconds} seconds\n" .
        "- **Average assignments per active second:** " . ($state->{active_seconds} > 0 ? sprintf('%.3f', Math::BigInt->new($state->{completed_assignments})->numify() / $state->{active_seconds}) : '0') . "\n" .
        "- **Started UTC:** $state->{started_utc}\n" .
        "- **Updated UTC:** $state->{updated_utc}\n\n" .
        "Prime classifications in this BigUint run are not, by themselves, rigorous primality proofs. The 256-bit scan is not complete until status is `complete`; unresolved candidates require separate proof or compositeness verification.\n";
    atomic_text("$root/final-report.md", $human);
}
sub partition_path { my ($b,$s,$e)=@_; return sprintf('%s/big-b%03d-a%s-e%s.jsonl',$root,$b,$s,$e); }
sub is_palindrome { my ($s)=@_; return defined($s) && $s eq reverse($s); }
sub mr_witness {
    my ($n,$base)=@_;
    return 0 if $n->bcmp(3)<0 || !$n->is_odd();
    my $nm1=$n->copy()->bdec();
    my $d=$nm1->copy(); my $s=0;
    while (!$d->is_odd()) { $d->brsft(1); $s++; }
    my $a=Math::BigInt->new($base)->bmod($n);
    my $x=$a->bmodpow($d,$n);
    return 0 if $x->is_one() || $x->bcmp($nm1)==0;
    for (1..$s-1) { $x->bmul($x)->bmod($n); return 0 if $x->bcmp($nm1)==0; }
    return 1;
}
sub valid_record {
    my ($line,$bits,$start,$end,$require_index)=@_;
    my $r=eval { decode_json($line) }; return unless ref($r) eq 'HASH';
    my $dec=$r->{value}; return unless defined($dec) && $dec =~ /\A[1-9][0-9]*\z/;
    my $n=Math::BigInt->new($dec);
    my $binary=$n->as_bin; $binary =~ s/^0b//;
    my $hex=$n->as_hex; $hex =~ s/^0x//;
    return unless $n->as_bin =~ /^0b1/ && length($binary)==$bits;
    return unless ($r->{binary}//'') eq $binary && ($r->{hex}//'') eq $hex;
    return unless ($r->{binary_bits}//0)==$bits && ($r->{hex_digits}//0)==length($hex);
    return unless ($r->{decimal_digits}//0)==length($dec);
    return unless is_palindrome($binary) && is_palindrome($hex) && is_palindrome($dec);
    if (defined($r->{assignment_index})) {
        my $idx=Math::BigInt->new("$r->{assignment_index}");
        return if $idx->bcmp($start)<0 || $idx->bcmp($end)>=0;
        open my $check, '-|', $bin, 'big-index', "$bits", "$dec" or return;
        my $actual=<$check>; my $ok=close($check);
        return unless $ok && defined($actual);
        chomp $actual;
        return unless $actual eq "$idx";
    } elsif ($require_index) { return; }
    my $class=$r->{classification}//'';
    if ($class eq 'prime') {
        return unless $n->bcmp(251)<=0;
        for (my $d=2; $d*$d <= $n->numify(); $d++) { return if $n->copy()->bmod($d)->is_zero(); }
        return $class;
    }
    if ($class eq 'composite') {
        my $ev=$r->{proof_evidence}//'';
        if ($ev =~ /^factor=([0-9]+)$/) {
            my $f=Math::BigInt->new($1);
            return if $f->bcmp(1)<=0 || !$n->copy()->bmod($f)->is_zero();
            return $class;
        }
        if ($ev =~ /^miller_rabin_base=([0-9]+)$/ && mr_witness($n,$1)) { return $class; }
        return;
    }
    return $class if $class eq 'unresolved' && defined($r->{proof_evidence});
    return;
}
sub file_stats {
    my ($path,$b,$s,$e) = @_;
    return unless -f $path;
    open my $f, '<', $path or return;
    my @lines = <$f>; close $f;
    return unless @lines;
    my $foot = $lines[-1];
    my $get = sub { my ($k)=@_; return $1 if $foot =~ /"\Q$k\E":"?([^",}\n]+)"?/; return; };
    return unless $foot =~ /"status":"completed"/;
    return unless ($get->('bits') // '') eq "$b";
    return unless ($get->('assignment_start') // '') eq "$s";
    return unless ($get->('assignment_end_exclusive') // '') eq "$e";
    return unless ($get->('assignment_space') // '') eq "$counts{$b}";
    my $gen = $get->('generated'); return unless defined($gen) && $gen eq $e->copy()->bsub($s)->bstr();
    my $dp = $get->('decimal_pass'); return unless defined($dp);
    my $version = $get->('format_version') // 1;
    my $rows = @lines - 1; return unless $rows == $dp;
    my ($pr,$co,$un)=(0,0,0);
    my %seen;
    for my $line (@lines[0..$#lines-1]) {
        my $class=valid_record($line,$b,$s,$e,$version>=2) or return;
        my $record=decode_json($line);
        return if $seen{$record->{value}}++;
        $pr++ if $class eq 'prime';
        $co++ if $class eq 'composite';
        $un++ if $class eq 'unresolved';
    }
    return {generated=>$gen, records=>$dp, primes=>$pr, composites=>$co, unresolved=>$un};
}
sub launch_batch {
    my ($b,$start,$end,$chunk) = @_;
    my @tasks;
    my $started = time();
    my $cur = $start->copy();
    while ($cur->bcmp($end) < 0 && @tasks < $threads) {
        my $stop = $cur->copy()->badd($chunk);
        $stop = $end->copy() if $stop->bcmp($end) > 0;
        my $path = partition_path($b,$cur,$stop);
        my $stats = file_stats($path,$b,$cur,$stop);
        if (!$stats) {
            unlink $path if -e $path;
            my $pid = fork(); die "fork: $!" unless defined $pid;
            if (!$pid) { exec {$bin} $bin, 'big-range', "$b", "$cur", "$stop", $path; die "exec $bin: $!"; }
        }
        push @tasks, [$cur->copy(),$stop->copy(),$path,$stats];
        $cur = $stop;
    }
    my $failed = 0;
    for my $task (@tasks) {
        next if $task->[3];
        my $pid = wait();
        if ($pid < 0) { $failed=1; next; }
        $failed=1 if ($? >> 8) != 0;
    }
    my $elapsed = time() - $started;
    die "BigInt partition batch failed; supervisor will retry from checkpoint\n" if $failed;
    my $finished = $cur;
    my $batch_assign = Math::BigInt->bzero;
    my @manifest_ranges;
    for my $task (@tasks) {
        my ($s,$e,$path) = @$task;
        my $stats = file_stats($path,$b,$s,$e) or die "invalid committed partition $path\n";
        $batch_assign->badd($stats->{generated});
        $state->{result_records} += $stats->{records};
        $state->{prime_records} += $stats->{primes};
        $state->{composite_records} += $stats->{composites};
        $state->{unresolved_records} += $stats->{unresolved};
        $state->{partitions}++;
        push @manifest_ranges, {assignment_start=>"$s", assignment_end_exclusive=>"$e", generated=>$stats->{generated}, decimal_pass=>$stats->{records}, status=>'completed', file=>$path};
    }
    my $manifest_path = sprintf('%s/manifest-b%03d-a%s-e%s.json', $root, $b, $start, $finished);
    atomic_json($manifest_path, {bits=>$b, batch_start=>"$start", batch_end_exclusive=>"$finished", status=>'completed', completion_utc=>utc(), ranges=>\@manifest_ranges});
    my $completed = Math::BigInt->new($state->{completed_assignments})->badd($batch_assign);
    $state->{completed_assignments} = "$completed";
    $state->{bits} = $b;
    $state->{next_start} = "$finished";
    $state->{batch_active_seconds} = $elapsed;
    save('partition_batch_completed');
}

save('coordinator_started');

while ($state->{phase} ne 'complete') {
    if ($state->{phase} eq 'compare_2_128') {
        system "$FindBin::Bin/compare-big-u128.sh", $root, $u128_root, "$root/comparison-2-128.json";
        die "128-bit result comparison failed\n" if $? != 0;
        $state->{phase}='above_128'; $state->{bits}=129; $state->{next_start}='0';
        save('u128_bigint_comparison_passed');
        next;
    }
    my $b = int($state->{bits});
    if (defined($test_max_bits) && $b > $test_max_bits) {
        $state->{status}='test_stopped'; $state->{phase}='test_stopped'; save('test_bit_limit_reached'); last;
    }
    if ($b > 256) { $state->{phase}='complete'; $state->{status}='complete'; save('search_complete'); last; }
    my $count = $counts{$b}->copy();
    my $cur = Math::BigInt->new($state->{next_start});
    if ($cur->bcmp($count) >= 0) {
        if ($b == 128 && $state->{phase} eq 'common_2_128') {
            $state->{phase}='compare_2_128'; save('common_range_complete'); next;
        }
        $state->{bits}=$b+1; $state->{next_start}='0'; save('bit_length_complete'); next;
    }
    my $chunk = Math::BigInt->new($b <= 128 ? 1_048_576 : 100_000_000);
    my $batch_end = $cur->copy()->badd($chunk->copy()->bmul($threads));
    $batch_end = $count->copy() if $batch_end->bcmp($count)>0;
    launch_batch($b,$cur,$batch_end,$chunk);
}
