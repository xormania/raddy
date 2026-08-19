<?php
declare(strict_types=1);

$path = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?: '/';
$query = [];
parse_str((string) (parse_url($_SERVER['REQUEST_URI'] ?? '', PHP_URL_QUERY) ?? ''), $query);

header('Content-Type: text/plain; charset=UTF-8');
if ($path === '/random') {
    echo bin2hex(random_bytes(16));
    return;
}
$name = isset($query['name']) ? (string) $query['name'] : 'xor';
echo 'Hello ' . $name;
