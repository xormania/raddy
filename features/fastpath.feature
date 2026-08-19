Feature: Fast path
  Warm instances are stolen from a pool. Teardown is not on the response path.

  @stage-5
  Scenario: Teardown completes after the response is fully sent
    Given a server on an ephemeral port serving the hello-symfony artifact
    When I GET "/hello?name=xor"
    Then the HTTP status is 200
    And the HTTP body is exactly "Hello xor"
    And guest teardown completed after the response was fully sent

  @stage-5
  Scenario: A burst above pool_min is refilled back to idle
    Given a snapshot server with pool_min 2 and pool_max 4
    When I GET "/hello?name=xor" 3 times
    Then the HTTP status is 200
    And the idle pool size returns to 2
