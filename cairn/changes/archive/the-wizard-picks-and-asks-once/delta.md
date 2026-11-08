---
cairn: delta
change: the-wizard-picks-and-asks-once
---

# Delta

## ADDED Requirements

### Requirement: Each domain gets its best option without a question
For each domain of the account, mail and sending, the wizard SHALL list the options discovery found, each a protocol, a server and a sign-in method (a password, an API token or OAuth 2.0). A discovered IMAP or SMTP server offering neither implicit TLS nor STARTTLS SHALL be dropped, and of the HTTP ones, a discovered server that is not `https` SHALL be dropped. Two options reading the same, protocol, server and sign-in method, SHALL be one, so a server offering OAuth through several grants is one OAuth option. Mail options SHALL be ranked the provider's own API first where a provider rule matched (the Gmail API, Microsoft Graph), then JMAP, then IMAP; the servers of one protocol implicit TLS before STARTTLS; and within one server OAuth, an API token, then a password when `ortie` is on the `PATH`, and a password, an API token, then OAuth otherwise. Sending SHALL go through mail when mail reads over JMAP, the Gmail API or Graph, which send over their own session; otherwise over the best discovered SMTP server, implicit TLS before STARTTLS, signing in as mail does when that server offers it; and nowhere when no SMTP server was discovered, the account being saved without sending rather than with a guessed `smtp.<domain>`. The first ranked option of each domain, Auto, SHALL be picked without a question.

#### Scenario: A JMAP server offering OAuth and a password
- GIVEN discovery finds JMAP with OAuth and a password, and IMAP with a password
- AND `ortie` is on the `PATH`
- WHEN the plan is made
- THEN mail is JMAP through OAuth and sending goes through mail

#### Scenario: The same server without Ortie
- GIVEN the same discovery and no `ortie` on the `PATH`
- WHEN the plan is made
- THEN mail is JMAP with a password

#### Scenario: A provider rule matched
- GIVEN an address a provider rule matches, offering its own mail API beside IMAP and SMTP
- WHEN the plan is made
- THEN mail is that API and sending goes through mail

#### Scenario: A plain server
- GIVEN discovery finds IMAP over implicit TLS, IMAP without encryption, and JMAP at an `http://` URL
- WHEN the plan is made
- THEN the IMAP server over implicit TLS is the only mail option

### Requirement: The review lists each pick in plain words
Before any sign-in, the wizard SHALL show one row per domain through pimalaya-cli's `review::prompt`, worded without jargon beyond the protocol name: `<Protocol> at <host>, with your password`, `, with an API token`, `, through your browser` (OAuth through Ortie) or `, with a token from a broker` (OAuth without Ortie); a provider API by its name, as `Gmail API, through your browser`; sending sharing mail's secret `, with mail's password`; sending over mail's session `Through mail`; a domain without a pick `Not set up`. Continue SHALL be the default and SHALL need at least one domain set up. Changing a domain SHALL list `Auto (<what Auto is>)`, every option found as `<Protocol> · <Sign-in method>`, the server URL appended only where two entries would otherwise read the same, `Custom server URL`, and for sending `Through mail` (when mail sends over its own session) and `No sending`; a pick SHALL return to the review.

#### Scenario: IMAP and SMTP of one domain
- GIVEN mail IMAP at imap.example.org and sending SMTP at smtp.example.org, both with a password
- WHEN the review shows
- THEN it reads `Mail     IMAP at imap.example.org, with your password` and `Sending  SMTP at smtp.example.org, with mail's password`

#### Scenario: One host over two transports
- GIVEN IMAP with a password on imap.example.org over implicit TLS and over STARTTLS
- WHEN mail is changed
- THEN the list reads `IMAP · Password · imaps://imap.example.org:993` and `IMAP · Password · imap://imap.example.org:143`

### Requirement: Nothing discovered asks for a server URL
When discovery yields no mail option for an address or a bare domain (an empty result, the deadline elapsing with nothing completed, or a resolver that does not answer), the wizard SHALL say that nothing could be discovered for the domain and ask `Server URL:`. An empty answer SHALL stop the wizard with a message stating it could not automatically discover a configuration for the input and inviting the user to write the account by hand from the documented sample configuration (linked), nothing being written. The same prompt SHALL follow the `Custom server URL` entry of a change, an empty answer there returning to the review.

#### Scenario: An unknown domain
- GIVEN an address whose domain discovery knows nothing of
- WHEN the lookup ends
- THEN the wizard says nothing could be discovered for the domain and asks `Server URL:`
- AND an empty answer stops it naming the documented sample, with no file written

### Requirement: A typed server URL names its protocol by its scheme
A typed server URL SHALL name its protocol by its scheme: `imaps://host[:port]` is IMAP over implicit TLS (993 by default), `imap://` IMAP over STARTTLS (143); `smtps://` SMTP over implicit TLS (465), `smtp://` SMTP over STARTTLS (587); `https://`, `jmaps://` and `jmap://` are JMAP over https. Encrypted servers only: an `http://` URL SHALL be refused naming it, and asked again, unless its host is this machine (`localhost` or a loopback address), where it is JMAP over http. A URL without a scheme, a scheme the domain does not take (SMTP for mail, anything but SMTP for sending) and a URL carrying a password SHALL be refused and asked again too. A user name in the URL is the login. The sign-in method SHALL then be picked from Password (the default) and API token. A typed IMAP server, when discovery found no SMTP server, SHALL be followed by `Sending server URL (empty for none):`, an empty answer leaving sending out.

#### Scenario: A cleartext server
- GIVEN the answer `http://mail.example.org`
- WHEN it is read
- THEN the wizard refuses it naming `http://mail.example.org`, saying only encrypted servers are accepted, and asks again

#### Scenario: A cleartext server on this machine
- GIVEN the answer `http://localhost:8080/jmap/session`
- WHEN it is read for mail
- THEN mail is JMAP at `http://localhost:8080/jmap/session`

#### Scenario: A bare IMAP host
- GIVEN the answer `imaps://mail.example.org`
- WHEN it is read
- THEN mail is IMAP over implicit TLS at mail.example.org:993

### Requirement: The sign-in mechanism is picked, not asked
The wizard SHALL NOT prompt for a SASL mechanism, an HTTP authentication scheme, the user id of the Gmail API or Microsoft Graph (`me`), or a login it knows. A password SHALL sign IMAP in with the best mechanism the unauthenticated CAPABILITY probe advertises, SCRAM-SHA-256, then PLAIN, then the LOGIN command, and PLAIN when the probe fails or advertises none of them; a token with OAUTHBEARER when advertised, else XOAUTH2. SMTP SHALL take PLAIN for a password and XOAUTH2 for a token, and JMAP Basic for a password and Bearer for a token. The login SHALL be the user name of a typed URL, else the advertised user name when it is an address, else the typed address, and SHALL be asked only when none of them is known. The mechanism stays editable in the configuration.

#### Scenario: A server advertising PLAIN and LOGIN
- GIVEN an IMAP server whose unauthenticated CAPABILITY advertises `AUTH=PLAIN` and no `LOGINDISABLED`
- WHEN it is signed in with a password
- THEN the account's IMAP SASL is PLAIN, and no mechanism was asked

### Requirement: Sign-ins are grouped by identity
After the review, the domains set up SHALL be signed in once per identity, in domain order: one OAuth sign-in per authorization server, covering every domain that picked it, and one password or API token per group of servers of one name, servers whose hosts are equal, or equal once their first label is dropped (`imap.example.org` and `smtp.example.org`), an IP address or a two-label host never losing one. A later group signing in with the same kind of secret on another host SHALL be asked `Use the same password for <host>?` (`API token` for a token), no by default, before its own prompt. A domain already signed in during the run SHALL NOT be asked again after a return to the review unless its pick changed.

#### Scenario: Mail and sending of one domain
- GIVEN mail at imap.example.org and sending at smtp.example.org, both with a password
- WHEN they sign in
- THEN one password prompt lists both servers

#### Scenario: Sending elsewhere
- GIVEN mail at imap.example.org and sending at smtp.example.net, both with a password
- WHEN mail signed in
- THEN the wizard asks `Use the same password for smtp.example.net?`, no by default, before a second prompt

### Requirement: A password is typed once, tested, then stored
A password or an API token SHALL be asked once, masked and without confirmation, through pimalaya-cli's `keyring::prompt_password`, which names every server it goes to and flags one off the address's domain. Each group SHALL be tested as soon as it is typed, through the connection tests under their spinners. Once a test passed, the wizard SHALL offer to store the secret once through pimalaya-cli's `keyring::prompt_store`, under the entry `himalaya-<account name>-<service>` (`imap`, `smtp`, `jmap`), the account name being the one the account is saved under, and the domains sharing the secret sharing the entry. The configuration SHALL hold the stored secret's read command, never the typed value unless raw storage was picked.

#### Scenario: A password stored in a keyring
- GIVEN mail and sending signed in with one password the tests accepted
- WHEN the user stores it in an installed keyring
- THEN the entry `himalaya-<account name>-imap` holds it, and both `imap.sasl` and `smtp.sasl` read it with the keyring's command

### Requirement: OAuth hands off to Ortie
Himalaya runs no OAuth 2.0 grant itself. With `ortie` on the `PATH`, an OAuth sign-in SHALL run pimalaya-cli's `ortie::sign_in` with the typed address (the domain when none was typed), the Ortie account `himalaya-<account name>` (suffixed with its first service when the account needs several OAuth sign-ins) and the services it covers (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`), the configuration reading the token with the command it answers. Without it, the wizard SHALL say in one line that a token broker issues the token and that Ortie can sign in for it, then offer the token brokers through pimalaya-cli's token picker.

#### Scenario: An API mail sign-in
- GIVEN mail over the Gmail API through OAuth, sending through mail, and `ortie` on the `PATH`
- WHEN mail signs in for the account `perso`
- THEN `ortie configure --name himalaya-perso --service gmail -- <address>` runs, then `ortie auth get -a himalaya-perso`
- AND the account's token command is `ortie token show -a himalaya-perso`

### Requirement: A refusal offers a way on
A failed sign-in or connection test SHALL be reported for its domain, the domains that passed being kept, and SHALL offer `Try again` (retype the secret, or sign in again), `Change the login` (ask the login, then retype; where the sign-in uses a login), `Leave <domain> out` (only when another domain is set up or still to come) and `Back to the review`.

#### Scenario: Sending refuses the password
- GIVEN mail and sending sharing one password, which IMAP accepts and SMTP refuses
- WHEN the refusal shows
- THEN mail stays signed in, and `Leave sending out` saves an account without sending

## MODIFIED Requirements

### Requirement: Input orients the flow
A single prompt SHALL accept an email address (or bare domain), a `scheme://` server URL, or a local folder path, and SHALL be the only question asked before discovery. An email, bare domain or server URL runs io-pim-discovery's parallel discovery; a folder is a local Maildir or m2dir. A server URL discovers from its host and its scheme narrows the mail options: `imap`/`imaps` keep IMAP (with `imaps` requiring implicit TLS), and `jmap`/`jmaps`/`http`/`https` keep JMAP; the provider APIs are dropped when a scheme is given, and any other scheme is refused before discovery. When no mail option is left, the URL itself SHALL be taken as the mail server, by the rules of a typed server URL.

#### Scenario: A server URL discovery does not know
- GIVEN the input `imaps://mail.example.org`
- WHEN discovery finds no IMAP server for mail.example.org
- THEN mail is IMAP over implicit TLS at mail.example.org:993, with no `Server URL:` prompt

### Requirement: Discovery is time-bounded
The parallel discovery run SHALL show `Looking up <domain>` and SHALL be bounded by a short deadline so a single unreachable endpoint (a firewalled port, a black-hole host) cannot stall the interactive wizard. Each mechanism runs independently; any that has not reported by the deadline is abandoned, and only what completed in time is offered. A DNS resolver that does not answer SHALL be reported and read as nothing discovered. When nothing completes, the wizard asks for a server URL (see "Nothing discovered asks for a server URL").

### Requirement: The prompted address is kept
When the wizard's single prompt is answered with an email address rather than a server URL or a folder path, that address SHALL be written as the generated account's `email`. A remote account's last question SHALL be `Your name (optional):`, written as the account's `display-name` when not empty and left out otherwise.

### Requirement: Connection tested before printing
Every backend of the account SHALL be tested before the fragment is printed, a remote one as soon as it is signed in and a local one by the account test, so a configuration that cannot connect is never generated. The printed fragment is compact: only the `[accounts.<name>]` table stays a section header, other tables flatten into dotted keys, and empty tables and defaulted values are dropped.

## REMOVED Requirements

### Requirement: One entry per service, then auth
Replaced by "Each domain gets its best option without a question", "The review lists each pick in plain words" and "The sign-in mechanism is picked, not asked".

### Requirement: OAuth folds into the API token
Replaced by "OAuth hands off to Ortie".

### Requirement: Per-protocol test and shared SMTP credentials
Replaced by "Sign-ins are grouped by identity", "A password is typed once, tested, then stored" and "A refusal offers a way on"; the rule against guessing an SMTP host moved to "Each domain gets its best option without a question".

### Requirement: Stop when nothing is discovered
Replaced by "Nothing discovered asks for a server URL".
