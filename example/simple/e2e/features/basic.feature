Feature: The simple demo showing how hydration behaves.

    Scenario: Showing the "fixed" demo does not error.
        Given I see the app
        When I access the link Fixed
        And I refresh the browser
        Then I see the bolded text is Hello World!
        And I find that the application is still working

    Scenario: Showing the "Hydration Error" may have errored
        Given I see the app
        When I access the link Hydration Error
        And I refresh the browser
        Then I find that the application has panicked
