Feature: HTTP ingress
  Bytes from a socket reach the executor and stream back.

  @stage-2
  Scenario: Toy echo over HTTP
    Given a server on an ephemeral port serving the "echo" guest
    When I POST "/echo" with body "hello"
    Then the HTTP status is 200
    And the HTTP body contains "hello"
    And the HTTP body contains "/echo"

  @stage-2
  Scenario: Saturation returns 503 and Retry-After
    Given the server concurrency is 1
    And a server on an ephemeral port serving the "spin" guest
    When I GET "/" without waiting for the response
    And I GET "/"
    Then the HTTP status is 503
    And the Retry-After header is "1"

  @stage-2
  Scenario: First body byte arrives while the guest is still producing
    Given a server on an ephemeral port serving the "slow_echo" guest
    When I GET "/" and read the first body byte
    Then at least one more body byte arrives after the first
