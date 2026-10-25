# Getting started

Revel builds a website without writing code: you place elements on the page, set them up in the panels, then publish. Everything happens on your computer; nothing is sent anywhere without your consent.

## Create or open a site

1. On Revel's home screen, choose **New site**.
2. Name it and pick a template: **Blank**, **Showcase** or **Blog**.
3. Revel opens the **Editor**. Your recent sites stay on the home screen.

A Revel site opens with **Open a site**. An existing site comes in with **Import**:

- **Folder**: a folder of HTML pages (hand-made, or an unzipped Webflow code export) or an Eleventy project. Each page keeps its address; the shared header and footer go into a layout, the styles into the Styles panel. Webflow export: add the collections' CSV files (CMS › collection › Export) or a Webflow API token to bring in the CMS content.
- **Link**: the address of a live site (made with Framer, Webflow or anything else). Revel reads its pages and copies its images, fonts and styles into the projects folder; appear animations become Revel animations.
- **Git repository**: paste a repository address (`https://github.com/account/site`, or a folder's: `…/tree/main/docs`). Revel copies it into the projects folder, history included, then imports it. A site that needs building (SvelteKit, Next, Astro, Hugo, Jekyll…) can be built by Revel (**Build**: the tools it needs are downloaded once, then kept in **Settings › Build tools**); you can also import its published site or the folder produced by its build. Your team's repository in a shared folder is picked with **Folder…**.
- **Figma**: the file link (Share › Copy link) and a personal access token (Figma › Settings › Security › Personal access tokens). Choose the frames: each one becomes a page, the first one is the home page.
- **WordPress**: the export file (Tools › Export › All content). Posts become a collection, pages keep their address, images are copied from the site.

At the end, the dialog shows what was brought in and the points to check, then opens the site in the editor.

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

**States**: the menu next to the class sets hover, pressed or focus. “Open (accordion)” styles an open accordion: the canvas opens it while you set it.

**Masonry**: the grid layout button stacks items of different heights in columns, without gaps; “Columns in turn” places them in order, one column after the other.

**Classes**: the **Classes** panel lists the site's classes and the elements that carry them. **Rename** changes the name everywhere (styles, elements, actions); the filter shows unused classes, which the trash button removes. Double-click a class in **Styles** to rename it.

**Site colors and fonts**: in a color picker, **+** adds the color to the site colors. **Settings › General › Colors** lists them all; **Dark mode** gives each one a value for visitors whose device uses a dark theme, and the moon button of the toolbar shows the site that way. Fonts are managed in **Settings › General › Fonts**.

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

A component is a piece of page reused everywhere (header, footer, card, service, tool). Right-click an element in the layers › **Create component**. Editing the component changes all its uses; each use can have its own texts, images and links (its **properties**, in **Styles**) and variant (hover, pressed…).

**Slot**: in the component, the **Content slot** switch of an element (a card's list of tools, a service's points) lets each use put its own content there. On the page, **Edit here** copies the component's content into the use to change it there only; **Back to the component content** undoes it.

The CMS is for content that has its own page (projects, articles); blocks of a page, even repeated (services, tools, rankings), are components placed on the page.

## Content (CMS)

1. Open **CMS** at the top. Create a **collection** (Projects, News…) and its **fields** (title, date, image, rich text…).
2. Add **items**: each one is a file of the site, also editable with Pages CMS.
3. In the editor, add a **Collection list**: one element per item. Select a text or an image of the list and link it to a field in **Styles › Linked to the entry**.
4. Each item has its **page**, made from a shared template created with the collection: set it once for all from **Pages**.

A long list can **show first** only a few items (list setting): the rest appears when the visitor reaches the bottom.

A linked date is written the way you want (October 16, 2026, 16.10.2026…); an element can be shown only when a field is set.

## Animations and actions

- **Interactions › Animations**: pick a moment (appear, text, hover, press, loop, scroll), then an animation. Set duration and delay; the ▶ button replays it.
- **Interactions › Actions**: a press, a hover or scrolling acts on an element (open a menu, make the header opaque, close a banner).
- **Settings › General › Page transition**: fade or slide from one page to the next.
- **Settings › General › Smooth scrolling**: the page, or the column under the pointer, glides to its destination with the wheel.
- **Add › Progress bar**: fills as the column that contains it, or the page, scrolls (“Follows” setting).
- **Follows the visible section** (element containing links to sections of the page): the link of the section being read gets the “Active” state.
- An action can target **the enclosing element of a class**: each card of a list opens on its own.

## Languages

**Localization** adds a language to the site. Texts are translated by hand or automatically, on your computer (language downloaded in **Settings › Automatic translation**). A machine translation is marked in purple until it is reviewed.

## Publish

1. **Settings › Publishing**: add a hosting (OVH, o2switch, Netlify, GitHub Pages…). Revel guides each step.
2. **Publish** at the top right sends the changed pages. A test version can come before the public site.
3. **Preview** opens the site as it will be published, animations and links included.

## Undo and versions

- **Ctrl+Z** undoes, **Ctrl+Shift+Z** redoes.
- **Settings › Versions** keeps the site's history: save a version, compare, go back.

## Work as a team

- **Settings › Versions › Team work**: choose **Shared folder** (a folder on the team's network, NAS or server: no account) or **Online repository** (GitHub, GitLab: usable anywhere). The window that turns it on compares both.
- Everyone works on their own copy of the site, then clicks **Sync**: the others' versions come in, theirs go out. Changes made in different places are combined.
- One place changed by two people: choose **Mine** or **Received** for each, then **Apply**.
- The others open the site with **Import › Git repository**, pasting the copied address or using **Folder…**. **Settings › Accounts › Your name** puts each person's name on their versions.
- Online repository: everyone has their own GitHub account, invited to the repository (Settings › Collaborators); GitHub forbids sharing an account.

## Revel updates

- Each time it starts, Revel checks for a newer version; a **Version x.y.z** button then appears at the top right.
- **Settings › Updates** installs it and restarts Revel. Each version is checked by its signature before it is installed.
- **Receive pre-releases** also offers test versions, published before the stable release.

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
