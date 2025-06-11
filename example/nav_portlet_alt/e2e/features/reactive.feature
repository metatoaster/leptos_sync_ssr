Feature: The navigation portlet is in fact reactive

    Scenario: Seeing the refresh inside Authors section work
        Given I see the app
        When I access the following links in the following order
            | Authors                 |
            | Bethany                 |
            | Articles by this author |
            | Beware of...            |
        # this is to allow time for the link to show up.
        Then I can see the following links
            | The top twenty...              |
        And I can see the following links under Navigation
            | On the practical nature of...  |
            | How to guide to...             |
            | The top ten...                 |
            | Why a city's infrastructure... |
            | The ultimate guide to...       |
            | The top hundred...             |
            | A quick summary on...          |
            | The top thousand...            |
            | Beware of...                   |
        But I will not find the following links under Navigation
            | Albert                  |
            | Bethany                 |
            | Carl                    |
            | Dorothy                 |

    Scenario: Seeing the refresh inside Article section work
        Given I see the app
        When I access the following links in the following order
            | Articles                |
            | The top ten...          |
            | dorothy                 |
        Then I can see the following links
            | Albert                  |
        And I can see the following links under Navigation
            | Bethany                 |
            | Carl                    |
            | Dorothy                 |
        But I will not find the following links under Navigation
            | The top twenty...              |
            | On the practical nature of...  |
            | How to guide to...             |
            | The top ten...                 |

    Scenario: Navigating between authors, the info is reactive
        Given I see the app
        When I access the following links in the following order
            | Authors                 |
            | Bethany                 |
            | Dorothy                 |
        And once I see the author overview is populated
        Then I can see the entity id is dorothy

    Scenario: Navigating between articles, the info is reactive
        Given I see the app
        When I access the following links in the following order
            | Articles           |
            | How to guide to... |
            | The top ten...     |
        And once I see the article view is populated
        Then I can see the entity id is 4
