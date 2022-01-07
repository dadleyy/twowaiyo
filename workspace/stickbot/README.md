## Stickbot

This a [rust] application is the http-based web api backend for [twowaiyo], using a simple REST api to allow players
to join, participate and leave games.


#### Developing Locally

The web backend currently relies on three third party services:

1. [mongodb] for peristing table and player state.
2. [redis] for the asynchronous background job queue.
3. [auth0] for handling the integration with third party OAuth providers.

At this time, free, hosted instances of mongodb and redis are available at [mongodb.com/cloud][mdbc] and [redislabs]
respectively. After creating the necessary accounts (or leveraging on-premise version of these), the `.env.example`
file located at the root of this repository can be used as a template for setting the credentials for authenticating
with these services.

Once the environment variables have been prepared, the main web process and background worker can be started using the
appropriate `cargo` aliases:


[rust]: https://www.rust-lang.org/
[twowaiyo]: https://github.com/dadleyy/twowaiyo
[mongodb]: https://www.mongodb.com/
[redis]: https://redis.io/
[mdbc]: https://www.mongodb.com/cloud
[redislabs]: https://app.redislabs.com/#/login
[auth0]: https://auth0.com/
