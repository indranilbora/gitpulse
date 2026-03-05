# AgentPulse Landing Page

Dedicated static landing page for AgentPulse.

## Contents

- `index.html`: semantic page structure and copy
- `styles.css`: Shiori-inspired visual system and responsive styling
- `main.js`: copy actions, command palette behavior, analytics events
- `assets/logo.svg`: copyable SVG logo
- `assets/og-image.png`: social preview image
- `assets/screenshots/*.png`: product screenshots used in demo surface

## Information Architecture

1. Header: centered brand and compact navigation
2. Hero: product title, value statement, primary install CTA
3. Demo: framed runtime screenshot surface
4. Features: two-column capability grid
5. Install: Homebrew + run-once cards with copy actions
6. Footer: attribution and project links

## Design Direction

- Warm, neutral palette inspired by `shiori.sh`
- Single light theme
- Rounded cards, soft borders, subtle depth
- Pill CTAs with minimum 44px hit targets
- Transform/opacity-only motion and reduced-motion support

## Local Preview

1. From repo root, run:

   ```bash
   python3 -m http.server 4173 --directory website
   ```

2. Open `http://127.0.0.1:4173`.

## Event Contract

The page emits these analytics events:

1. `lp_view`
2. `lp_click_install_primary`

No other landing analytics events are emitted by `website/main.js`.

## Vercel Deploy (Static)

1. Create a Vercel project from this repository.
2. Set **Root Directory** to `website`.
3. Leave **Build Command** empty.
4. Leave **Output Directory** empty (static root).
5. Enable **Vercel Web Analytics** in project settings.
6. Deploy to the default Vercel subdomain first.

## Launch Validation Checklist

1. Check Chrome, Safari, and Firefox rendering.
2. Confirm hero primary CTA jumps to `#install`.
3. Confirm both copy buttons copy full command blocks.
4. Confirm `Cmd/Ctrl+K` opens command palette and `Esc` closes it.
5. Confirm keyboard navigation reaches all links and buttons.
6. Confirm only `lp_view` and `lp_click_install_primary` fire.
7. Confirm no clipping or overflow at 1440px and 375px widths.

## Canonical project links

- Docs: `https://github.com/indranilbora/agentpulse/blob/master/README.md`
- Support: `https://github.com/indranilbora/agentpulse/blob/master/SUPPORT.md`
- Security: `https://github.com/indranilbora/agentpulse/blob/master/SECURITY.md`
