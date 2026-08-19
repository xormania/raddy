Feature: Snapshot resume
  A Wizer snapshot is resumed per request. Entropy and time are not frozen.

  @stage-4
  Scenario: A request is served from a resumed snapshot
    Given a server on an ephemeral port serving the hello-symfony artifact
    When I GET "/hello?name=xor"
    Then the HTTP status is 200
    And the HTTP body is exactly "Hello xor"
    And the request was served by a resumed instance

  @stage-4
  Scenario: Snapshot does not freeze entropy
    Given a server on an ephemeral port serving the hello-symfony artifact
    When I GET "/random" twice
    Then the HTTP status is 200
    And the two response bodies differ

  @stage-4
  Scenario: Request time tracks the host clock
    Given a server on an ephemeral port serving the hello-symfony artifact
    When I GET "/hello?name=xor"
    Then the HTTP status is 200
    And REQUEST_TIME is within 5 seconds of the host clock
