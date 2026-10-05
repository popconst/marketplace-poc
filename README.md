# WePush take-home: a creator marketplace

Advertisers create campaigns. Creators see the campaigns that fit them and bid. When bidding ends, a worker closes the campaign and picks the winners within the budget. All of it works through the UI.

## Run it

You only need Docker (Docker Desktop, or any Docker with Compose). From the repo folder:

```sh
./start.sh
```

It runs `docker compose --profile seed up --build`, or `docker-compose` if that's what you have. The first start compiles the Rust backend, which takes a few minutes. Then open http://localhost:8080. There's no login. You pick an advertiser or a creator account to act as. If port 8080 is taken, start it with `WEB_HOST_PORT=8090 ./start.sh` instead.

The `seed` profile fills an empty database with demo data once the migrations have run: 25 advertisers, each with one open campaign and one or two drafts, and 50,000 creators with about 74,000 TikTok and Instagram accounts. If the database already has data, it's left alone, so restarting keeps your changes.

Without `--profile seed` you get an empty marketplace. The API can't create advertisers or creators, so that's only useful if you bring your own data.

`docker compose down -v` (or `docker-compose down -v`) deletes everything. Ports and settings go in `.env` (see `.env.example` and [Configuration](#configuration)).

Compose also turns on the demo tools (`DEV_TOOLS`). You get a Debug menu that makes bots bid and moves a campaign's deadline to 10 seconds from now, and you can publish a campaign that closes in 5 minutes.

### A quick tour

1. Pick **Advertiser**, choose a brand and create a campaign. The panel on the right estimates what the budget will buy as you fill it in. Publish it with any deadline.
2. Open the **Debug** menu and click **Simulate creator bids**. The campaign page now shows what those bids would buy.
3. Go back to the home page, pick **Creator**, filter the accounts by the campaign's platform, country, genre and size, and **Connect as** one of them. The campaign shows up in its feed with a suggested bid. Place a bid and watch "If bidding closed now".
4. Go back to the campaign page, open the **Debug** menu and click **Close in 10 seconds**. Ten seconds later the worker closes the campaign, just as it would at a real deadline, and the page switches to the results by itself: the winners, and the lost bids with their reasons. The creator sees the outcome under **My bids**. **Close campaign now** in the same menu closes it right away, without the worker.

## What's where

```mermaid
flowchart LR
    FE["Frontend<br/>SvelteKit, in the browser"] -- "HTTP /api" --> API["API<br/>Rust"]
    API -- "reads and writes" --> DB[("Postgres")]
    DB -- "every 30 s: campaigns<br/>due in the next 60 s" --> W["Worker<br/>Rust"]
    W -- "closes each one<br/>at its deadline" --> DB
```

The frontend only talks to the API. The API and the worker never talk to each other: they share the database, and closing locks the campaign so the two can't clash.

| Path | What |
|---|---|
| `backend/` | Rust workspace. `marketplace` has the rules for pricing, matching, winner selection and the estimate, with no I/O. `db` has the settings, row types, shared SQL, publishing, closing and the `migrate` binary. `api` is the axum HTTP server, `worker` the closing job and `seed` the demo data |
| `backend/migrations/` | SQL migrations, built into the binaries |
| `frontend/` | SvelteKit single-page app (Svelte 5, Tailwind CSS 4, daisyUI 5), served by nginx, which also proxies `/api` |
| `docker-compose.yml` | `postgres`, `migrate` (runs once), `api`, `worker`, `web`, and `seed` (only with the `seed` profile, runs once) |
| `start.sh` | Builds and starts everything with the demo data, with whichever Compose is installed |

## How matching, pricing and closing work

```mermaid
flowchart LR
    A["Advertiser publishes<br/>a campaign"] --> B["Matching creators<br/>see it in their feed"] --> C["Creators bid<br/>until the deadline"] --> D["Closing job picks<br/>winners within budget"]
```

The numbers below are defaults you can change (see [Configuration](#configuration)). When a campaign is published, its deal rules are copied onto it, so changing a setting later never touches a campaign that's already live or closed.

### Matching: who sees a campaign

```mermaid
flowchart LR
    A["Creator account"] --> T{"Fits the<br/>targeting?"}
    T -->|yes| S{"Campaign buys<br/>this size?"}
    S -->|yes| B{"Min bid within<br/>25% of budget?"}
    B -->|yes| F["In the feed,<br/>ranked by match score"]
    T -->|no| H["Not shown"]
    S -->|no| H
    B -->|no| H
```

A creator account sees a campaign when three things are true:

- **It fits the targeting.** Same platform, brand safe, and in one of the campaign's countries, languages and genres. An empty list means any. For genres, one match is enough.
- **The campaign buys its size.** Sizes go by average views per post: nano 500 to 3K, micro 3K to 30K, macro 30K to 300K, mega above 300K. Advertisers can pick any mix, and nano, micro and macro is the default.
- **Its lowest bid fits in a quarter of the budget.** No single bid may take more than 25%, so big accounts don't see small campaigns.

The feed is sorted by a match score from 0 to 100. It reflects the campaign's sliders for engagement, content quality and track record, plus how many of the campaign's genres the account covers. Each slider has five steps, from "not important" to "very important". Track record is WePush's reliability score: does the creator post on time and pass review? The match score counts again when the winners are picked.

### Pricing: what a creator is offered

Example: an account with 10,000 views per post, on a campaign that buys nano, micro and macro accounts with an €18 target CPM.

| | Rule | Example |
|---|---|---|
| Usual rate | €90 + €10 per 1,000 views | €190 |
| Offer | Target CPM ÷ what these sizes usually cost, never below 0.5 | €18 ÷ €19 = 0.95 |
| Suggested bid | Usual rate × offer, at least the lowest bid | €180 |
| Lowest bid | Half the usual rate, at least €90 | €95 |
| Highest bid | Where the views needed to get paid reach 60% of the average | €325.71 |
| Views to get paid | 35% of the average at the usual rate, more as the bid goes up | 3,316 at €180 |
| Take-home | Bid minus WePush's 15% commission | €153 at €180 |

The offer is fixed when the campaign is published. The suggested bid is also capped at a quarter of the budget, so it can win. The bid limits never depend on the budget, because creators don't see it.

### Closing: who wins

At the deadline, the worker picks the winners:

1. Bids above 25% of the budget, or above the account's highest bid, are out.
2. The budget is split evenly between the chosen sizes, starting with the biggest.
3. In each size, bids are ranked by value: price per 1,000 views ÷ match score. A better match can charge a bit more and still win.
4. 15% of the size's money goes first to its weaker matches (below the median score), so the sliders don't shut them out completely.
5. The rest goes down the ranking until the money runs out. A bid that doesn't fit is skipped, and a cheaper one below it can still win.
6. Whatever a size doesn't spend moves to the next size down. What's left at the end goes back to the advertiser.

Every lost bid gets one of three reasons:

| Reason | Meaning |
|---|---|
| Outranked | Every winner in its size was better value |
| Too expensive for the money left | A cheaper bid with worse value won after it |
| Over the limit | Above 25% of the budget, or above the account's highest bid |

The draft estimate, the live view of an open campaign and the creator's "would win" check all run this same selection.

<details>
<summary>Exact rules</summary>

| Rule | Formula |
|---|---|
| Size group | By average views per post: starter < 500 ≤ nano < 3K ≤ micro < 30K ≤ macro < 300K ≤ mega. Campaigns can't pick starters |
| Usual rate | €90 + €10 per 1,000 average views |
| Budget split | Even across the picked sizes. Leftover cents go to the smallest |
| Usual CPM of a mix | What the picked sizes cost per 1,000 views when each spends its share on typical accounts (1.5K, 10K, 90K and 1M views) at their usual rates. That's the harmonic mean of their CPMs: about €30 for nano + micro, €19 for nano + micro + macro, €14 for micro + macro, €10.50 for macro + mega |
| Offer | Target CPM ÷ usual CPM of the mix, at least 0.5. Set and frozen when the campaign is published |
| Min bid | 50% of the usual rate, at least €90 |
| Campaign rate | Usual rate × offer, at least the min bid |
| Suggested bid | The campaign rate, capped at the max bid and at 25% of the budget |
| Max bid | 60 ÷ 35 × usual rate, where the minimum views reach 60% of the average. It depends only on the account, never on the budget |
| Budget share | A bid above 25% of the budget loses at closing. Advertisers don't set a euro cap. WePush sets a per-video price cap through support on request, and this fixed share replaces it |
| Minimum views to get paid | Average views × 35% × bid ÷ usual rate, rounded up |
| Expected CPM | Bid ÷ average views × 1,000 |
| Shown in the feed | Fits the targeting, its size is picked, and its min bid is at most 25% of the budget. The estimate's matching accounts count exactly these |
| Match score | Weighted mean of the factors the campaign cares about, × 100. Engagement is the rate ÷ 10%, capped at 1. Content quality and track record are the score ÷ 100. Genre is the share of the campaign's genres the account covers, with a weight of 0.5 |
| Value for fit | Bid ÷ average views × 1,000 × 100 ÷ match score |
| Closing | Sizes go from biggest to smallest. Each spends its own share plus what the bigger size left. 15% of a size's share goes first to bids below the size's median match score. The rest goes to the best value for fit first, each bid only if it fits. What the smallest size leaves goes back to the advertiser |
| Take-home | Bid − WePush's 15% commission, rounded half up to the cent |

Equal money per size doesn't mean equal videos. With €10,000 and an €18 target across nano, micro and macro, you get about 33 nano, 18 micro and 3 macro videos, roughly 54 videos and 500K views. Mega needs a big budget: a typical mega account's usual rate is about €10,000, and the estimate warns you when a size can't afford a single creator.

</details>

## Assumptions

- The rate card (€90 plus €10 per 1,000 views) is fitted to published numbers. WePush pays about €100 for a nano video with 500 to 3,000 views (€95 to €120 here). Its case studies show €128 to €211 for 6,000 to 17,000 views (€150 to €260 here). The industry quotes about €10 CPM for an account with a million followers (€10,090 for a million views here).
- There's no per-video data, so I assumed views per video follow a log-normal spread with σ ≈ 1. Then about 85% of ordinary videos reach 35% of the median, and about 70% reach 60%. That's where the 35% and the 60% come from.
- The 25% cap per creator follows WePush's own rule. They capped one creator at 25% of a budget, and later at 15%.
- The size boundaries and the typical views per size (1,500, 10,000, 90,000 and 1,000,000) are my own.
- The offer never goes below half the usual rate. Below that, a target says more about the advertiser's guess than about what creators will accept.
- The 15% commission is my choice. WePush's take rate was reported at about 25% (somewhere between 20 and 30%), built into the creator price. `COMMISSION_BPS` changes it for campaigns published after the change.
- The estimates only count average views. A nano video costs at least €90, so nano reach comes out at €30 to €180 per 1,000 views. WePush sells small creators on volume and on the few videos that go viral, and these numbers leave that upside out.
- The demo data is made up too. Its distributions are at the top of `backend/seed/src/creators.rs`, and the genres that go together are in `backend/seed/src/genre.rs`.

## Configuration

Each binary reads its settings at startup, from environment variables or the matching command-line flags (`--help` lists them). Everything has a default. If the settings don't make sense together, the binary won't start and names the setting that's wrong. With Compose, put them in `.env` (see `.env.example`). Shares are in basis points (2500 means 25%) and money is in euro cents.

**Deal rules.** The API and the seeder read these. They're copied onto each campaign when it's published, so a change only affects campaigns published afterwards. The worker uses the copy on the campaign.

| Variable | Default | Meaning |
|---|---|---|
| `COMMISSION_BPS` | 1500 | WePush's commission on each winning bid (15%) |
| `USUAL_RATE_BASE_CENTS` | 9000 | The fixed part of the usual rate (€90), the cost of making a video. Also the lowest bid anyone can make |
| `USUAL_RATE_PER_1000_VIEWS_CENTS` | 1000 | What each 1,000 average views add to the usual rate (€10) |
| `FAIR_PAY_FLOOR_BPS` | 5000 | The min bid as a share of the usual rate (50%) |
| `MAX_BID_SHARE_OF_BUDGET_BPS` | 2500 | The most one winning bid can take of the budget (25%). The minimum budget follows from it (€360) |
| `VIEWS_TO_GET_PAID_BPS` | 3500 | The minimum views to get paid at the usual rate, as a share of the average views (35%) |
| `LIKELY_VIEWS_LIMIT_BPS` | 6000 | The highest minimum a bid can promise, as a share of the average views (60%). It sets the max bid at 6000 ÷ 3500, about 1.71 times the usual rate |
| `DISCOVERY_BPS` | 1500 | The share of each size's money that goes first to weaker matches (15%) |

**Campaign limits.** The API checks drafts against these when they're saved or published. They aren't copied onto campaigns, because they don't change a published deal. The seeder uses the lowest offer and the default posting window too.

| Variable | Default | Meaning |
|---|---|---|
| `MIN_OFFER_BPS` | 5000 | The lowest offer, as a share of the usual rate (0.5). Only the resulting offer is frozen |
| `MIN_BIDDING_HOURS` | 24 | The earliest bidding deadline, in hours after publishing. Ignored with `DEV_TOOLS`, so a demo campaign can close minutes after publishing |
| `MAX_BIDDING_HOURS` | 720 | The latest bidding deadline, in hours after publishing (30 days) |
| `POSTING_WINDOW_DEFAULT_DAYS` | 3 | The posting window a new draft starts with. Winners post within this many days of the deadline |
| `POSTING_WINDOW_MAX_DAYS` | 7 | The longest posting window an advertiser can pick |

**Operations.**

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | required | Postgres connection URL, for every binary |
| `API_BIND_ADDR` | `0.0.0.0:3000` | Where the API listens |
| `DATABASE_MAX_CONNECTIONS` | 10 for the API, 2 for the worker | Connection pool size. The seeder and `migrate` use one connection |
| `DEV_TOOLS` | `false` (Compose: `true`) | Turns on the demo routes under `/api/dev`: bots that bid with matching accounts, and closing a campaign now or in 10 seconds. It also lifts `MIN_BIDDING_HOURS`. `GET /api/options` reports it as `devTools`, and the frontend only shows the demo controls when it's on. Off in production |
| `WORKER_REFRESH_SECS` | 30 (Compose: 5) | How often the worker reloads the coming deadlines. It still closes each campaign right at its deadline. A deadline moved closer, as "Close in 10 seconds" does, is picked up on the next reload |
| `CLOSE_MAX_ATTEMPTS` | 5 | Failed attempts to close a campaign before the worker marks it `failed` and stops trying |
| `RUST_LOG` | `info` | Log filter, in the syntax of tracing's `EnvFilter` |
| `POSTGRES_HOST_PORT`, `WEB_HOST_PORT` | 5433, 8080 | Compose only: the host ports for Postgres and the web app |

At startup the binaries check that:

- every share is between 0 and 10,000, and the budget share and the views to get paid are above 0
- the base fee and the rate per 1,000 views are above 0
- `LIKELY_VIEWS_LIMIT_BPS` is above `VIEWS_TO_GET_PAID_BPS`
- `MIN_OFFER_BPS` is at least `FAIR_PAY_FLOOR_BPS`
- `MIN_BIDDING_HOURS` is at least 1 and below `MAX_BIDDING_HOURS`
- `POSTING_WINDOW_DEFAULT_DAYS` is between 1 and `POSTING_WINDOW_MAX_DAYS`
- `CLOSE_MAX_ATTEMPTS` is at least 1

A few values stay in the code. The size boundaries are part of the product's vocabulary, and changing them would regroup bids that are already placed. The typical views per size and the match score's constants are calibration. The 5-day window for counting views, the slider steps and the text limits only affect what people see.

## Running it in production

Here's how I'd run it.

- **Build.** CI builds two images. The backend image holds the `api`, `worker`, `migrate` and `seed` binaries and runs as a non-root user. The web image is nginx serving the built frontend and proxying `/api`. CI also runs the backend checks against a throwaway Postgres (every test creates its own database) and the frontend checks.
- **Database.** Managed Postgres with backups, point-in-time recovery and a standby. The migrations are built into the binaries, and each deploy runs `migrate` once, as a job, before the new API and worker roll out. From the first release on, migrations only get added, never edited. Right now I edit them in place (see [Develop](#develop)).
- **API.** It keeps no state, so it scales out behind a load balancer that handles TLS. `GET /api/healthz` is the liveness check. It never touches the database, so a database outage doesn't restart every instance. `GET /api/readyz` is the readiness check and does look at the database. `DEV_TOOLS` stays off.
- **Worker.** A small, always-on deployment. It keeps the next two refresh periods' deadlines in memory and closes each campaign right at its deadline. The database stays the only source of truth, so a restart or an extra replica just loads the deadlines again. Several replicas are safe. Closing locks the campaign and only closes it if it's still open, and bids hold the campaign while they're saved, so a campaign closes exactly once and no bid sneaks in during the close. A campaign that keeps failing is marked `failed` after `CLOSE_MAX_ATTEMPTS`, with its last error saved, and someone has to look at it. I'd alert on that, and on the oldest campaign still waiting to close. Failed attempts from every replica count, so with several replicas a campaign hits the limit sooner.
- **Live updates.** For now the pages poll. The live view of an open campaign reloads every 15 seconds, and any page reloads half a second after a deadline it shows, because closing happens on schedule. Next I'd have the API push "bids changed" events over a WebSocket and let pages reload when one arrives. With more than one API instance, they'd need a shared channel for that, such as Redis pub/sub or a hosted service.
- **Configuration.** Environment variables per environment, from the deployment's config and secret store (`DATABASE_URL` is a secret). A bad value stops the binary at startup, so a broken rollout fails its readiness check instead of mispricing campaigns. A change to a deal rule only affects campaigns published after it.
- **Observability.** The binaries log through `tracing`. In production I'd log JSON and add metrics: request latency and errors, how long closing takes, campaigns closed and failed, and the age of the oldest campaign waiting to close.
- **Seeder.** Never runs in production.

## Develop

```sh
docker compose up -d postgres
export DATABASE_URL=postgres://wepush:wepush@localhost:5433/wepush

cd backend
cargo run --bin migrate
cargo run --bin api -- --dev-tools    # http://localhost:3000
cargo run --bin worker

cd ../frontend
npm install
npm run dev                           # http://localhost:5173, proxies /api to :3000
```

While the schema is still changing, there's one migration per table in `backend/migrations/`, and I edit them in place instead of adding new ones. After changing one, reset the local database with `docker compose down -v` and start again.

Checks (the backend tests need `DATABASE_URL`, and each test gets its own throwaway database):

```sh
cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd frontend && npm run check && npm run lint
```

### Seed data

`backend/seed` fills a migrated database with fake advertisers and campaigns (one open for three days and one or two drafts each), plus creators and their TikTok and Instagram accounts with languages and genres (a main genre and up to three related ones). It doesn't run migrations. Compose's `seed` profile runs it with `--if-empty`.

```sh
cd backend
cargo run -p seed --release -- --reset
```

- By default it creates 25 advertisers, one per row of its campaign terms, and 50,000 creators (about 74,000 accounts). For a scale test, `--creators 15000000` loads 100 times the roughly 150,000 creators WePush reports, about 22 million accounts. Check the table below before you try it.
- It refuses a database that already has creators or advertisers. `--reset` first deletes them and everything that depends on them, including all campaigns and bids (genres, countries and languages stay). `--if-empty` leaves such a database alone and exits successfully.
- It publishes the open campaigns the way the API does, with settings from the same environment variables, so their deal rules are frozen on them.
- The same `--seed` and counts always give the same rows and ids (with the same `Cargo.lock`). Only the timestamps change. They're relative to when you run it, so the data always looks recent.
- The creator distributions are at the top of `backend/seed/src/creators.rs`, the related genres are in `genre.rs`, and the advertisers' categories, campaign pitches and terms are in `data.rs`. Views have a long tail on purpose, up to 20 million per post (about 200 accounts above a million at the default size), to show how pricing handles outliers. An account with 7 million views per post has a usual rate of about €70,000, which a campaign's limits then cap.
- Rows are generated as they're sent, streamed with `COPY` in chunks of 100,000 creators inside one transaction. The seeder stays under 70 MB of memory at any size.

Measured on an M1 Pro, with Postgres 18 in a Docker VM (4 CPUs, 8 GB) on default settings:

| Creators | Accounts | Time | Database size |
|---|---|---|---|
| 10,000 | 14,837 | under 1 s | 29 MB |
| 1,000,000 | 1,478,949 | 120 s | 1.4 GB |
| 15,000,000 (extrapolated) | about 22 million | about 45 min | about 21 GB |

I didn't run the full-size row. Its size is 15 times the 1M run. At the 1M run's speed it would take 30 minutes, but loading slows down as the indexes grow, so expect around 45. Leave at least 30 GB of free disk for the database and its write-ahead log.
