Feature: Calls out to the public section of the api 

  Scenario: check that the server time and local time are in a margin of error
    When the server time is requested
    Then a valid JSON response is retunred
    And the response is not cached
    And the response has no error messages
    And the system time is in a margin of 1 sec
    And the unixtime field corresponds with the rfc1123
