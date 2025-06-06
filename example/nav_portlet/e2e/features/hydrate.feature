Feature: The navigation portlet hydrates correctly

    Scenario: Seeing the refresh inside Authors section work
        Given I see the app
        When I access the following links in the following order
            | Authors                 |
            | Albert                  |
            | Articles by this author |
        And I refresh the browser
        Then I find that the application is still working
        And I can see the following links under Navigation
            | Albert                  |
            | Bethany                 |
            | Carl                    |
            | Dorothy                 |

    Scenario: Seeing the refresh inside Article section work
        Given I see the app
        When I access the following links in the following order
            | Articles                |
            | The top ten...          |
        And I refresh the browser
        Then I find that the application is still working
        And I can see the following links under Navigation
            | The top twenty...              |
            | On the practical nature of...  |
            | How to guide to...             |
            | The top ten...                 |
            | Why a city's infrastructure... |
            | The ultimate guide to...       |
            | The top hundred...             |
            | A quick summary on...          |
            | The top thousand...            |
            | Beware of...                   |

    Scenario: Navigation to home should drop portlets
        Given I see the app
        And I access the following links in the following order
            | Authors                 |
            | Albert                  |
        When I refresh the browser
        And I access the link Home
        Then I will find the Navigation section disappear

    Scenario: Navigating between authors should maintain reactive info
        Given I see the app
        And I access the following links in the following order
            | Authors                 |
            | Albert                  |
        When I refresh the browser
        And I access the link Dorothy
        And once I see the author overview is populated
        Then I can see the entity id is dorothy
