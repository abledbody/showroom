# Showroom
Showroom is a discovery-first UI overlay for a terminal emulator intended to make the experience for new Linux users easier and more intuitive.

## Principles
### The terminal is a fallback
It should always be available and work as normal, but it should not be directly required to interact with the shell.
### Strictly value-add
Even the most die-hard terminal poweruser should have no objections to using Showroom as their terminal, and Showroom should add a little bit of value to the terminal in even the most poorly configured circumstances.
### Discovery-first
If the user knows what they want from the computer, it should be easy to figure out how to do that quickly and directly from the terminal.
### Heirarchy of importance
Not all commands, arguments, or flags are used equally. While no use case should disappear from the UI entirely, Showroom should present opinions about what's most important for the user. Media players and terminals are both user interface. Media players put the play button front and center, why should the terminal be presented uniformly?
### Invalidity should be a decision
Not all strings are valid commands, so Showroom should not present the option to make invalid choices with equal weight. Leave the ability to make decision that Showroom does not think are valid to the terminal UI itself.
### Opinionated configuration, customizable opinions
Showroom's philosophy is not that anyone's opinion on how to present options is correct, but that there should be any opinion at all. Presentation follows a heirarchy, lowest to highest, from Showroom's own default configurations, to a command's bundled configuration, to user-chosen community resource packs, to the user's own on-machine, personal configuration.