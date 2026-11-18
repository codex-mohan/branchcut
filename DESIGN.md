---
name: Branchcut
description: A filesystem diagnostics desk for query planning and visible traversal work.
colors:
  night-canvas: "#080b09"
  night-surface: "#101512"
  light-paper: "#f3fbf6"
  white-surface: "#ffffff"
  signal-mint: "#57ef97"
  mint-highlight: "#9cffc2"
  lime-verdict: "#c9ff55"
  trace-cyan: "#61d9ff"
  prune-amber: "#ffb32d"
  dark-ink: "#f7fff9"
  dark-muted: "#a6b8ad"
  dark-line: "#2a362f"
  terminal-black: "#030605"
typography:
  display:
    fontFamily: "Archivo, sans-serif"
    fontSize: "clamp(4rem, 7.4vw, 7.35rem)"
    fontWeight: 560
    lineHeight: 0.88
    letterSpacing: "-0.04em"
  headline:
    fontFamily: "Archivo, sans-serif"
    fontSize: "clamp(2.65rem, 5.4vw, 5.1rem)"
    fontWeight: 540
    lineHeight: 0.98
    letterSpacing: "-0.035em"
  title:
    fontFamily: "Archivo, sans-serif"
    fontSize: "clamp(1.65rem, 2.7vw, 2.5rem)"
    fontWeight: 590
    lineHeight: 1.03
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Archivo, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.7
  label:
    fontFamily: "Fragment Mono, monospace"
    fontSize: "0.65rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0.07em"
rounded:
  square: "0px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
  section: "clamp(5.5rem, 9vw, 9rem)"
components:
  button-primary:
    backgroundColor: "{colors.signal-mint}"
    textColor: "{colors.terminal-black}"
    typography: "{typography.body}"
    rounded: "{rounded.square}"
    padding: "12px 16px"
    height: "48px"
  button-primary-hover:
    backgroundColor: "{colors.mint-highlight}"
    textColor: "{colors.terminal-black}"
    rounded: "{rounded.square}"
  button-secondary:
    backgroundColor: "{colors.night-surface}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.square}"
    padding: "12px 16px"
    height: "48px"
  terminal-window:
    backgroundColor: "{colors.terminal-black}"
    textColor: "{colors.dark-ink}"
    rounded: "{rounded.square}"
  benchmark-accent:
    backgroundColor: "{colors.lime-verdict}"
    textColor: "{colors.terminal-black}"
    rounded: "{rounded.square}"
---

# Design System: Branchcut

## Overview

**Creative North Star: "The Filesystem Diagnostic Desk"**

Branchcut looks like an instrument for understanding work that has already happened: a query enters one side, a filesystem tree moves through the middle, and measured evidence leaves the other. The interface is square, direct, and information-dense without feeling cramped. It should feel alert and engineered, not like a generic developer-tool dashboard.

Dark surfaces are the default because the page is often open beside a terminal, but the palette avoids a single muddy green field. Neutral near-black establishes focus; luminous mint identifies the selected route; chartreuse marks decisive evidence; cyan separates comparison tools; orange marks pruned work. Bright blocks interrupt the dark field at useful boundaries.

**Key Characteristics:**

- Zero-radius geometry everywhere.
- Graph-paper structure tied to filesystem traversal.
- Dark-first surfaces with high-energy signal colors.
- Honest data visualization and visible system boundaries.
- Monospace reserved for commands, paths, and measurements.

## Colors

The palette combines clean neutral darkness with four functional signals: mint for Branchcut, lime for decisive evidence, cyan for comparators, and amber for pruned work.

### Primary

- **Signal Mint** (`{colors.signal-mint}`): Primary actions, selected tabs, active tree states, and Branchcut benchmark bars.
- **Mint Highlight** (`{colors.mint-highlight}`): Hover, focus emphasis, and foreground green on the darkest surfaces.
- **Lime Verdict** (`{colors.lime-verdict}`): Final calls to action, fact rail in dark mode, and the terminal end of the Branchcut benchmark bar.

### Secondary

- **Trace Cyan** (`{colors.trace-cyan}`): Comparator benchmark bars and secondary diagnostic data.
- **Prune Amber** (`{colors.prune-amber}`): Excluded directories, cut paths, and warnings.

### Neutral

- **Night Canvas** (`{colors.night-canvas}`): Default page field outside elevated surfaces.
- **Night Surface** (`{colors.night-surface}`): Sections and data panels.
- **Terminal Black** (`{colors.terminal-black}`): Terminal and code surfaces.
- **Dark Ink** (`{colors.dark-ink}`): Primary dark-theme text.
- **Dark Muted** (`{colors.dark-muted}`): Supporting dark-theme text.
- **Dark Line** (`{colors.dark-line}`): Rules, grid lines, and physical panel seams.

**The Functional Signal Rule.** Green means Branchcut or a viable path. Cyan means another measured tool. Amber means work removed or requiring attention. Do not reuse those colors decoratively.

## Typography

**Display Font:** Archivo (with sans-serif fallback)
**Body Font:** Archivo (with sans-serif fallback)
**Label/Mono Font:** Fragment Mono (with monospace fallback)

**Character:** Archivo gives the page a compact, editorial engineering voice with strong scale changes. Fragment Mono identifies commands, paths, units, versions, and operational labels without turning the whole interface into a terminal costume.

### Hierarchy

- **Display** (560, clamp 4–7.35rem, 0.88): The landing headline only.
- **Headline** (540, clamp 2.65–5.1rem, 0.98): Major section questions.
- **Title** (590, clamp 1.65–2.5rem, 1.03): Mechanism and workflow titles.
- **Body** (400, 1rem, 1.7): Explanations capped near 58ch in the hero and 54ch in section introductions.
- **Label** (400, 0.65rem, 0.07em): Plan fields, benchmark metadata, and small diagnostic labels.

**The Signal Type Rule.** Use monospace when the text is code, a filesystem path, a unit, a version, or measured data. Never use it to imitate technical atmosphere.

## Layout

The desktop layout is a 1380px maximum instrument frame with a 64px graph-paper grid. The hero is a two-column split: narrative at left and interactive planner at right. Content sections alternate large explanatory spreads with full-width evidence surfaces. Borders provide the hierarchy, so spacing does not rely on card gaps.

Major sections use a 5.5–9rem vertical rhythm and 1.25–4rem horizontal insets. At 1120px the hero stacks. At 760px tables become stacked labels and values, the fact rail becomes a single column, and the query tabs become vertical. The mobile composition keeps the same reading order and visible primary action.

## Elevation & Depth

The system is flat by default. Borders, tonal blocks, and intersecting rules communicate structure. Only the planner, install selector, workflow console, and benchmark instrument receive a soft directional shadow so they read as temporary tools placed on the diagnostic desk.

### Shadow Vocabulary

- **Instrument Shadow** (`0 26px 70px -42px rgb(0 74 44 / 0.5)` light; `0 30px 80px -44px rgb(0 0 0 / 0.94)` dark): Floating planner and code surfaces only.

**The Hard Rule Rule.** A visible section boundary uses a 1px rule. Do not replace it with a shadow or a rounded container.

## Shapes

Every surface, control, tab, field, dialog, badge, and code window uses a 0px border radius. Geometry comes from horizontal and vertical lines, right angles, grid intersections, and branch paths. Avoid circles except for native control semantics such as the window-status dot; use squares for the visual system's own status marks.

## Components

### Buttons

- **Shape:** Square (0px radius), one-pixel border where secondary.
- **Primary:** Signal Mint fill, near-black text, 12px by 16px padding, 48px minimum height.
- **Hover / Focus:** Primary shifts toward Mint Highlight; focus uses a 2px Signal Mint outline with a 3px offset.
- **Secondary:** Transparent or Night Surface fill with Dark Line border; hover moves the border to a signal color.

### Tabs

- **Style:** Equal-width square controls divided by one-pixel rules.
- **State:** Selected tabs gain a solid signal-tinted field and a two-pixel inset bottom rule; unselected tabs remain neutral.

### Cards / Containers

- **Corner Style:** None (0px).
- **Background:** Night Surface or Terminal Black.
- **Shadow Strategy:** Instrument Shadow only for interactive tools.
- **Border:** One-pixel Dark Line.
- **Internal Padding:** 16–44px depending on data density.

### Inputs / Fields

- **Style:** Transparent or Night Surface fill, one-pixel rule, square corners.
- **Focus:** Two-pixel Signal Mint outline.
- **Error / Disabled:** Amber for recoverable issues; reduced opacity and blocked input for disabled states.

### Navigation

Sticky 56px top navigation with compact Archivo labels, square search input, theme toggle, and GitHub action. Mobile replaces the links with a menu trigger. All controls square off against rules rather than floating as pills.

### Filesystem Planner

The signature component combines a command row, query presets, plan metadata, and a directory tree. A single scanning line moves through the tree to show traversal. Active paths are mint; matches use square terminals; pruned branches turn amber and receive a slash mark. The demo is labeled illustrative.

### Benchmark Instrument

The benchmark switcher uses tabs over a fixed data surface. Branchcut bars run from Signal Mint to Lime Verdict; comparator bars use Trace Cyan. Exact timings remain tabular and visible, and method limitations sit directly beneath the chart.

## Do's and Don'ts

### Do:

- **Do** keep every corner at 0px, including dialogs, inputs, tabs, and badges.
- **Do** use color as a functional signal and preserve the green-cyan-amber distinctions.
- **Do** label synthetic traversal geometry and workload-specific benchmarks explicitly.
- **Do** use visible rules and grid intersections to structure the page.
- **Do** keep keyboard focus and reduced-motion behavior visible and complete.

### Don't:

- **Don't** tint every dark surface with green; neutral darkness provides the contrast the signal colors require.
- **Don't** add pill buttons, rounded cards, glow halos, or soft corners.
- **Don't** use gradients outside measured data visualization.
- **Don't** introduce a second icon family or a stronger display typeface.
- **Don't** turn plan data into unsupported product claims.
