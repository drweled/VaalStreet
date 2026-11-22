<img width="369" height="231" alt="image" src="https://github.com/user-attachments/assets/8861971a-60a3-427d-9957-ad2a5fb7e1a7" />


See how much your Path of Exile stash is worth.

Sign in with your Path of Exile account, pick your stash tabs, and Vaal Street adds up
everything in them using poe.ninja prices. It also keeps a history, so you can see your
wealth go up (or down) over time, and it can track how much profit a farming session made.

## What gets priced

Currency, fragments, threads, essences, oils, fossils, delirium orbs, catalysts,
divination cards, gems, unique items, maps (normal, blighted and unique maps),
wombgifts, runegrafts, corpses and more.

Your **Maps** and **Unique** stash tabs are supported too.

## Features

- Opens over the game with **Ctrl+W**. Esc, Ctrl+W or a click outside puts it away.
- Total worth of the tabs you pick, with a breakdown per tab
- Snapshots: Lets you calculate your investment and deduct it from revenue to see how much profit you make.

## Your account

You sign in on pathofexile.com's own login page, inside the app. Vaal Street never sees
your password, and nothing about you is collected or sent anywhere. Your history and
settings stay on your PC (`%APPDATA%\VaalStreet`).

Reading your stash uses your account's request limit on pathofexile.com. Vaal Street always
stays a few requests under it and waits between requests, so syncing won't get you timed out.

## Download

Get `VaalStreet_<version>_Setup.exe` from [Releases](https://github.com/drweled/VaalStreet/releases).

### "Windows protected your PC"?

Windows may show a blue box the first time you run the setup. That's Windows
SmartScreen, and it shows it for any new program that not many people have
downloaded yet. It isn't a sign that something is wrong with Vaal Street.

Vaal Street is safe: all of its code is right here for anyone to read, every
release is built from it, and it collects nothing about you. Click
**More info**, then **Run anyway**.

## Build from source

Needs Node.js 20+, Rust and the Tauri CLI (`cargo install tauri-cli`):

```powershell
powershell -ExecutionPolicy Bypass -File build.ps1 -NoUpdater
```

The setup ends up in `dist\`.

## License

MIT. Not affiliated with Grinding Gear Games.
