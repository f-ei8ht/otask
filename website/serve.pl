#!/usr/bin/env perl
use strict;
use warnings;
use IO::Socket::INET;
use POSIX qw(SIGCHLD SIG_DFL);
use File::Basename qw(basename);

my $PORT = 5000;
my $ROOT = do { my $d = $0; $d =~ s|/[^/]+$||; $d };

my %MIME = (
    html => 'text/html; charset=utf-8',
    css  => 'text/css',
    js   => 'application/javascript',
    svg  => 'image/svg+xml',
    png  => 'image/png',
    jpg  => 'image/jpeg',
    ico  => 'image/x-icon',
    txt  => 'text/plain',
);

$SIG{CHLD} = 'IGNORE';

my $server = IO::Socket::INET->new(
    LocalPort => $PORT,
    Type      => SOCK_STREAM,
    Reuse     => 1,
    Listen    => 10,
) or die "Cannot bind port $PORT: $!\n";

print "Serving $ROOT on port $PORT\n";
$| = 1;

while (my $client = $server->accept) {
    my $pid = fork;
    if (!defined $pid) { close $client; next; }
    if ($pid) { close $client; next; }

    # child
    my $request = '';
    while (my $line = <$client>) {
        $request .= $line;
        last if $line eq "\r\n" || $line eq "\n";
    }

    my ($method, $path) = $request =~ m|^(\w+)\s+(/[^\s]*)|;
    $path //= '/';
    $path =~ s|\?.*||;
    $path =~ s|%([0-9A-Fa-f]{2})|chr(hex($1))|ge;
    $path =~ s|/\.\./|/|g;

    $path = '/index.html' if $path eq '/';
    my $file = $ROOT . $path;

    if (-f $file) {
        my ($ext) = $file =~ /\.(\w+)$/;
        my $mime = $MIME{lc($ext // '')} // 'application/octet-stream';
        open my $fh, '<:raw', $file or do {
            print $client "HTTP/1.1 500 Error\r\nContent-Length: 5\r\n\r\nError";
            close $client; exit;
        };
        my $body = do { local $/; <$fh> };
        close $fh;
        my $len = length $body;
        print $client "HTTP/1.1 200 OK\r\nContent-Type: $mime\r\nContent-Length: $len\r\nConnection: close\r\n\r\n$body";
    } else {
        my $body = "404 Not Found: $path";
        my $len  = length $body;
        print $client "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: $len\r\n\r\n$body";
    }

    close $client;
    exit;
}
