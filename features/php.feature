Feature: PHP guest, cold
  A fresh PHP instance serves each request. No snapshot.

  @stage-3
  Scenario: PHP hello over HTTP
    Given a server on an ephemeral port serving the PHP app
    When I GET "/hello"
    Then the HTTP status is 200
    And the HTTP body is exactly "Hello"

  @stage-3
  Scenario: POST body echo is bytes-exact
    Given a server on an ephemeral port serving the PHP app
    When I POST "/echo" with body "hello world"
    Then the HTTP status is 200
    And the HTTP body is exactly "hello world"

  @stage-3
  Scenario: Repeated Set-Cookie headers keep order
    Given a server on an ephemeral port serving the PHP app
    When I GET "/cookies"
    Then the HTTP status is 200
    And the Set-Cookie headers are "a=1" then "b=2"
