# Getting started

Revel builds a website without writing code: you place elements on the page, set them up in the panels, then publish. Everything happens on your computer; nothing is sent anywhere without your consent.

## Create or open a site

1. On Revel's home screen, choose **New site**.
2. Name it and pick a template: **Blank**, **Showcase** or **Blog**.
3. Revel opens the **Editor**. Your recent sites stay on the home screen.

A Revel site opens with **Open a site**; an existing Eleventy site comes in with **Import**: its pages and content are kept.

## The editor

- **In the middle, the canvas**: the page as it will be published. A click selects an element; Shift+click adds another one to the selection.
- **On the left**: **Pages** (site pages and collection pages), **Components**, **Media**, then **Layers** (the page tree) and **Add**.
- **On the right**: **Styles** (look of the selected element) and **Interactions** (animations and actions).
- **At the top**: the screens (desktop, tablet, phone), zoom, preview and **Publish**.

Panels can move: drag a tab to another column, or onto the canvas to make it a window.

## Add elements

1. Select where to add (a section, a block).
2. In **Add**, double-click an element or drag it onto the page.
3. **Ready-made blocks** (page header, features, FAQ, form…) add a whole section.

To move an element, drag it on the page or in **Layers**. Alt+drag makes a copy.

## Set the look

The look is set in **Styles**, per **class**: every element with the same class changes together, as in Webflow.

1. Select the element, then type a class name in the field at the top of **Styles** (for example `card`).
2. Set layout, spacing, size, text, background, border, effects.
3. A **blue** label is set here; an **orange** label is inherited (from a wider screen or from the normal state). Click a label to see where the value comes from, or to remove it.

**Screens**: pick tablet or phone at the top; what you set then only applies to that screen and smaller ones.

**States**: the menu next to the class sets hover, pressed or focus.

**Site colors and fonts**: in a color picker, **+** adds the color to the site colors; fonts are managed in **Settings › Fonts**.

## Text

- **Double-click** a text to type on the page. **Enter** confirms, **Shift+Enter** adds a line break, **Esc** cancels.
- Select words: a bar offers **bold**, *italic*, **link** and **span** (a part of the text styled on its own).
- The **Text** field at the top of **Styles** also edits the selected element's text.

## Images and media

1. Add an **Image**, then **Choose an image** in **Styles**.
2. Import your files in **Media** (drag and drop). Revel makes light images for each screen.
3. Write an **alternative text**: it describes the image to people who cannot see it.

**Videos**, **Lottie animations** (`.json` or `.lottie` file) and documents are added the same way.

## Components

A component is a piece of page reused everywhere (header, footer, card). Right-click an element in the layers › **Create component**. Editing the component changes all its uses; each use can have its own texts and variant (hover, pressed…).

## Content (CMS)

1. Open **CMS** at the top. Create a **collection** (Projects, News…) and its **fields** (title, date, image, rich text…).
2. Add **items**: each one is a file of the site, also editable with Pages CMS.
3. In the editor, add a **Collection list**: one element per item. Select a text or an image of the list and link it to a field in **Styles › Linked to the entry**.
4. In **Pages**, create **each item's page**: its template is set once for all.

A linked date is written the way you want (October 16, 2026, 16.10.2026…); an element can be shown only when a field is set.

## Animations and actions

- **Interactions › Animations**: pick a moment (appear, text, hover, press, loop, scroll), then an animation. Set duration and delay; the ▶ button replays it.
- **Interactions › Actions**: a press, a hover or scrolling acts on an element (open a menu, make the header opaque, close a banner).
- **Settings › General › Page transition**: fade or slide from one page to the next.

## Languages

**Localization** adds a language to the site. Texts are translated by hand or automatically, on your computer (language downloaded in **Settings › Automatic translation**). A machine translation is marked in purple until it is reviewed.

## Publish

1. **Settings › Publishing**: add a hosting (OVH, o2switch, Netlify, GitHub Pages…). Revel guides each step.
2. **Publish** at the top right sends the changed pages. A test version can come before the public site.
3. **Preview** opens the site as it will be published, animations and links included.

## Undo and versions

- **Ctrl+Z** undoes, **Ctrl+Shift+Z** redoes.
- **Settings › Versions** keeps the site's history: save a version, compare, go back.

## Useful shortcuts

| Action | Shortcut |
|---|---|
| Search a command | Ctrl+K |
| Undo, redo | Ctrl+Z, Ctrl+Shift+Z |
| Duplicate, delete | Ctrl+D, Del |
| Parent, child element | Shift+Enter, Enter |
| Wrap in a block | Ctrl+G |
| Fit zoom, 100 % | Shift+1, Shift+0 |
| Hide panels | Ctrl+\ |

The full list is in the **?** menu of the editor bar.
