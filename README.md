```
  ____ ___ _____   ____ _____ _  _____ ____
 / ___|_ _|_   _| / ___|_   _/ \|_   _/ ___|
| |  _ | |  | |   \___ \ | |/ _ \ | | \___ \
| |_| || |  | |    ___) || / ___ \| |  ___) |
 \____|___| |_|   |____/ |_/_/   \_\_| |____/
```

Fun stats about any git repo, in a colorful interactive terminal scoreboard.

## Install

```sh
cargo install --path .
```

## Usage

Run it inside any git repository:

```sh
git-stats
```

Navigate with **←/→** to switch tabs, **↑/↓** to scroll, **q** to quit.

## Stats

- **Committers** — who commits the most
- **Churn** — the most-changed files (lock/vendored files ignored)
- **Biggest** — the single largest commit
- **Night Owls** — when commits happen, plus biggest night owls & earliest birds
- **Streaks** — longest consecutive-day commit streaks
- **Words** — commit types, top phrases, and a word cloud
- **Ownership** — file owners and bus-factor warnings
- **Vitals** — totals, authors, age, and pace
- **Oops** — commits that own a mistake (wip, typo, revert…)
- **Busiest** — the single busiest day
- **Battlefield** — files touched by the most authors

Authors with slightly different names/emails (including GitHub noreply addresses) are merged into one.

## Options

| Flag | Description |
|------|-------------|
| `--limit <N>` | Analyze at most N most-recent commits |
| `--since <YYYY-MM-DD>` | Only commits on/after this date |
| `--include-generated` | Include lock/generated/vendored files in churn & battlefield |
| `--jobs <N>` | Worker threads for collection (default: min(cores, 8)) |
| `--no-color` | Disable colors |
