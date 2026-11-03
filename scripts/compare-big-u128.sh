#!/usr/bin/env bash
set -euo pipefail
BIG_ROOT="$(cd "${1:?BigInt result directory required}" && pwd -P)"
U128_ROOT="$(cd "${2:?u128 result directory required}" && pwd -P)"
perl - "$BIG_ROOT" "$U128_ROOT" "${3:-}" <<'PERL'
use strict;
use warnings;
use JSON::PP qw(encode_json);
use Math::BigInt;
my ($big,$u128,$report)=@ARGV;
sub read_values {
    my ($dir,$glob)=@_;
    my %found;
    my $records=0;
    for my $path (glob "$dir/$glob") {
        open my $f,'<',$path or die "open $path: $!";
        while (my $line=<$f>) {
            next unless $line =~ /"value":"(\d+)"/;
            my $v=$1;
            my ($bin)=$line =~ /"binary":"([01]+)"/;
            my ($hex)=$line =~ /"hex":"([0-9a-f]+)"/;
            my ($class)=$line =~ /"classification":"([a-z]+)"/;
            die "missing representation for $v in $path\n" unless defined($bin) && defined($hex);
            die "duplicate value $v in $dir\n" if exists $found{$v};
            $found{$v}={binary=>$bin,hex=>$hex,classification=>$class};
            $records++;
        }
        close $f;
    }
    return (\%found,$records);
}
my ($a,$an)=read_values($big,'big-b???-*.jsonl');
my ($b,$bn)=read_values($u128,'part-b*-*.jsonl');
my @missing=grep {!exists $a->{$_}} keys %$b;
my @extra=grep {!exists $b->{$_}} keys %$a;
my @representation=grep {exists $b->{$_} && ($a->{$_}{binary} ne $b->{$_}{binary} || $a->{$_}{hex} ne $b->{$_}{hex})} keys %$a;
my @classification=grep {exists $b->{$_} && ($a->{$_}{classification} // '') ne ($b->{$_}{classification} // '')} keys %$a;
my $ok=!@missing && !@extra && !@representation && !@classification && $an==$bn;
my @values=sort { length($a) <=> length($b) || $a cmp $b } keys %$a;
my $out={status=>$ok?'passed':'failed',timestamp_utc=>scalar(gmtime()).' UTC',bigint_records=>$an,u128_records=>$bn,missing_from_bigint=>\@missing,extra_in_bigint=>\@extra,representation_mismatches=>\@representation,classification_mismatches=>\@classification,values=>\@values};
my $json=encode_json($out)."\n"; print $json; if (defined $report && length $report) { my $tmp="$report.tmp.$$"; open my $o,">",$tmp or die $!; print {$o} $json; close $o or die $!; rename $tmp,$report or die $!; }
exit($ok?0:1);
PERL
