<?php
declare(strict_types=1);

$path = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
if (!is_string($path) || $path === '') {
    $path = '/';
}

switch ($path) {
    case '/hello':
        header('Content-Type: text/plain; charset=UTF-8');
        echo 'Hello';
        break;
    case '/echo':
        header('Content-Type: application/octet-stream');
        echo file_get_contents('php://input');
        break;
    case '/cookies':
        header('Set-Cookie: a=1');
        header('Set-Cookie: b=2', false);
        header('Content-Type: text/plain; charset=UTF-8');
        echo 'ok';
        break;
    default:
        http_response_code(404);
        header('Content-Type: text/plain; charset=UTF-8');
        echo 'not found';
}
