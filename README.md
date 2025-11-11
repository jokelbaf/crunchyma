<p align="center">
    <img src="https://cdn.jokelbaf.dev/crunchyma/logo-circle.png?" width="160" height="160" alt=""/>
    <h1 align="center">Crunchyma</h1>
</p>

<p align="center">
    An unofficial Crunchyroll Telegram bot written in Rust. Primarily designed to post new episode releases to <a href="https://t.me/crunchyroll_releases">my Telegram channel</a>.
</p>

## Development

To build and run the bot locally, you can simply use Cargo with nightly toolchain:

1. Clone the repository:
```bash
git clone https://github.com/jokelbaf/crunchyma.git
cd crunchyma
```
2. Configure the required environment variables. See [.env.example](.env.example) for reference.
3. Build and run the bot:
```bash
cargo build
cargo run
```

## Deployment

The bot can be deployed using Docker. A `Dockerfile` and `docker-compose.yml` are provided for easy deployment:

```bash
docker-compose up -d
```

## Acknowledgements

Great thanks to the authors of the following libraries used in this project:
- [teloxide](https://crates.io/crates/teloxide) - Telegram bot framework for Rust.
- [crunchyroll-rs](https://crates.io/crates/crunchyroll-rs) - Unofficial Crunchyroll API client for Rust.

## License

The project is distributed under the MIT License. See [LICENSE](LICENSE.md) for details.
