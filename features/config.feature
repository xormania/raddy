Feature: Configuration
  Raddy layers compiled defaults, a TOML file, environment variables, and CLI flags.

  @stage-0
  Scenario: An environment override is visible in --print-config
    Given the environment variable "RADDY_SERVER_LISTEN" is "0.0.0.0:9999"
    When I run raddy with "--print-config"
    Then the printed config parses as TOML
    And the printed server.listen is "0.0.0.0:9999"
    And the printed config round-trips through the TOML parser
