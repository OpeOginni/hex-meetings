# Hex Meetings site

The landing page for [Hex Meetings](https://github.com/OpeOginni/hex-meetings), served at https://hex-meetings.opeoginni.dev. Static Astro, no framework. Run the commands below from `site/`.

```sh
bun install
bun --bun astro dev      # http://localhost:4321
bun --bun astro build    # writes dist/
```

The Download button links to `releases/latest/download/Hex-Meetings-macOS-arm64.dmg`, so it follows each new release as long as the asset keeps that name. Screens in `src/assets/` are native captures of the app (fictional meetings).

## Deploy on Vercel

Import the repo in Vercel and set the Root Directory to `site`; it detects Astro from there and needs no other settings. Then add the domain `hex-meetings.opeoginni.dev` under Project, Settings, Domains, and add the CNAME record Vercel shows (`hex-meetings` to `cname.vercel-dns.com`) at your DNS provider.
