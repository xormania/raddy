Feature: Toy guest execution
  The executor runs a C guest through the raddy ABI.

  @stage-1
  Scenario: The echo guest reflects head and body
    Given the toy "echo" guest is loaded
    When I execute a GET "/echo" with body "hello"
    Then the execute result is a response
    And the response status is 200
    And the response body contains "/echo"
    And the response body contains "hello"

  @stage-1
  Scenario: A guest that never emits a head hits the epoch deadline
    Given the toy "spin" guest is loaded
    And the executor deadline is 50 ms
    When I execute a GET "/spin" with body ""
    Then the execute result is DeadlinePreHead

  @stage-1
  Scenario: A trapping guest is reported as a trap
    Given the toy "trap" guest is loaded
    When I execute a GET "/trap" with body ""
    Then the execute result is a trap
