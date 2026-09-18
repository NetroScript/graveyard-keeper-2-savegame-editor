# Visual style guide

The editor follows the game's interface palette and construction: blue-gray panels, brown title bars, yellow-gold active text, pale gray labels and layered metal frames. It contains no game screenshots, extracted assets or runtime icon downloads.

## Palette and surfaces

Shared tokens live in `src/lib/style.css`. Use `--surface` (#292a33) for panels, `--deep` (#20232c) for recessed inputs, `--metal` (#66636a) for outer frames, `--gold` (#d8a91f) for active controls, `--cream` (#ddd2bd) for prominent text and `--red` (#a62824) for destructive actions. Header bands use the game's muted brown around #795735. Secondary text must remain readable against its surface; disabled controls also retain their labels.

Frames use one thin border in a slightly brighter tone than their background. Corners are square. Components do not combine outlines and shadows, and the interface does not use drop or inset shadows. Very low-opacity repeating CSS textures add depth without competing with content. Avoid glossy effects, rounded cards, bright saturated backgrounds and ornamental graphics copied from the game.

## Typography and spacing

Use the locally bundled Roboto family for headings, labels, controls and prose. Use 700 weight for headings and 400 or 500 for controls and body text. Interface copy is direct and neutral. Do not add slogans, decorative eyebrow text or narrative descriptions. Technical identifiers use monospace and wrap rather than overflowing.

Use an 8 px spacing rhythm: 8 px between actions, 16 px between related fields, 24 px panel padding/gaps and 32 px workspace margins. Body text is normally 14 px; hints 12 px; page titles 25–30 px. The persisted interface-scale setting scales the workspace's base text; title and compact-caption sizes remain deliberate visual anchors.

## Navigation and hierarchy

The wide layout has a labeled left rail: Load Saves, open documents and Settings at the bottom. An ochre band distinguishes Save Editor and Save Inspector. Category navigation sits below it. The active item uses a distinct full background, a brighter border around the complete control and gold text. Do not use a thick line on one edge as the selection indicator. Unsaved changes have a dot and an accessible label.

Each open save retains its mounted view, local drafts and scroll containers. Hidden views use the HTML `hidden` attribute so they leave both layout and keyboard navigation. Future categories display an explicit Coming later label and an explanatory empty state.

## Controls and interaction

Inputs are recessed charcoal with a metal edge. Primary actions use muted gold; destructive actions use red. Use short verbs such as Apply, Save, Remove and Reload. Grouped edits submit on Enter or Apply as one transaction. Draft text remains local until applied, and saving remains a separate action.

Keyboard focus uses the same full-border, background and text-color change as other interaction states. Every icon-only action needs an accessible name. Hover brightens borders, pressed controls shift one pixel, and disabled controls retain a clear shape. Modal decisions use native dialogs to provide focus containment and Escape behavior. Inline errors describe the reason an operation was rejected.

## Icons

Use Phosphor icons through `~icons/ph/<icon-name>`. Vite's `unplugin-icons` compiles them from the installed `@iconify-json/ph` package. Standard control icons are 19 px; rail branding may use a larger icon. Match stroke weight and avoid mixing unrelated icon families. No icon CDN is needed.

## Responsive layout

Above 1050 px, General uses two columns and Inspector splits tree/details. Below that threshold, panels stack. Below 700 px, the rail becomes a drawer with a labeled menu button and dismissal scrim. Category navigation can scroll horizontally. Forms, metadata paths and identifiers wrap; the page must not acquire horizontal scrolling. Check 390 px and 1280 px viewports after layout changes. Honor reduced-motion preferences.
