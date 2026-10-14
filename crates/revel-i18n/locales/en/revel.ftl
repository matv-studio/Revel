app-title = Revel
app-tagline = Visual website builder
app-version = Version { $version }
app-stage = Stage { $stage }: modular skeleton

menu-file = File
menu-site = Site
menu-publish = Publish
menu-help = Help

language = Language

deploy-warn-no-301 = This host cannot send real 301 redirects: { $count } redirect(s) will become client-side redirects, which search engines handle less well.
deploy-warn-no-php = This host does not run PHP: { $count } PHP file(s) (for example the contact form) will not work.
deploy-warn-no-headers = This host does not allow custom HTTP headers (indexing, cache and security headers).
deploy-warn-no-htaccess = This host ignores .htaccess files: its rules (HTTPS, redirects, access) will not apply.
deploy-warn-site-too-big = The site ({ $size } MB) exceeds this host's limit ({ $max } MB).
deploy-warn-file-too-big = A file ({ $size } MB) exceeds this host's per-file limit ({ $max } MB).
deploy-warn-too-many-files = The site has { $count } files; this host accepts at most { $max }.
deploy-warn-public-repo = On a free plan, this host requires the source repository to be public.

home-title = Sites
home-empty = No site yet. Open a folder containing a Revel site, or import an Eleventy project.
action-open = Open a site
action-import = Import an Eleventy project
action-new = New site
action-forget = Remove from list
action-cancel = Cancel
action-save = Save
action-delete = Delete
new-site-name = Site name
import-overwrite = This folder already contains a Revel site. Importing again replaces its structure (pages, layouts, collections); content and images are not changed.
import-done = Import finished
import-warnings = Points to check
site-back = All sites
site-pages = Pages
site-layouts = Layouts
site-components = Components
site-collections = Collections
site-entries = { $count ->
        [one] { $count } entry
       *[other] { $count } entries
    }
build-title = Generate
build-env = Environment
build-images = Responsive images (AVIF, WebP)
build-run = Generate the site
build-running = Generating…
build-phase-pages = Pages
build-phase-copies = Files
build-phase-images = Images { $done }/{ $total }
build-done = { $generated } pages generated, { $copied } files copied, images: { $produced } new, { $cached } reused, in { $seconds } s.
build-warnings = Warnings
build-failed = Generation failed. { $message }
preview-open = Preview
publish-title = Publish
publish-empty = No hosting for this site yet. Add an SFTP server (OVH, o2switch…) or GitHub Pages to publish in one click.
publish-add = Add hosting
publish-name = Name
publish-env = Environment generated
publish-host = SFTP server
publish-port = Port
publish-user = Login
publish-remote-dir = Folder on the server
publish-password = Password
publish-password-saved = Already stored in the system keychain. Leave empty to keep it.
publish-check = Check
publish-run = Publish
publish-planned = { $upload ->
        [one] { $upload } file to send
       *[other] { $upload } files to send
    } ({ $size } MB), { $delete } to delete, { $total } files on the site.
publish-progress = Sending: { $done } / { $total } { $total ->
        [one] file
       *[other] files
    }
publish-done = { $uploaded ->
        [0] { $deleted ->
            [0] Nothing to publish: the online site is already up to date.
           *[other] Published: { $deleted } { $deleted ->
                [one] file deleted
               *[other] files deleted
            }.
        }
       *[other] Published: { $uploaded } { $uploaded ->
            [one] file sent
           *[other] files sent
        }, { $deleted } deleted.
    }
publish-dry-done = { $upload ->
        [0] { $delete ->
            [0] Check finished: the online site is already up to date.
           *[other] Check finished, nothing was changed on the server. The next publication will delete { $delete } { $delete ->
                [one] file
               *[other] files
            }.
        }
       *[other] Check finished, nothing was changed on the server. The next publication will send { $upload } { $upload ->
            [one] file
           *[other] files
        } ({ $size } MB) and delete { $delete }.
    }
publish-key-recorded = First connection: the server key is recorded and will be required for future connections. Compare its fingerprint with the one shown by your host:
publish-failed = Publication failed. { $message }
publish-warnings = Before publishing
error = Error: { $message }
nav-sites = Sites
nav-recent = Recent
nav-all-sites = All sites
nav-overview = Overview
nav-build = Generate
nav-publish = Publish
nav-editor = Editor
# Visual editor (step 2)
editor-loading = Opening the site in the editor…
editor-canvas = Page canvas
editor-page = Displayed page
editor-pages-fixed = Pages
editor-breakpoints = Breakpoints
editor-bp-desktop = Desktop (base)
editor-bp-max = Up to { $width } px
editor-bp-min = From { $width } px
editor-zoom = Zoom
editor-zoom-fit = Fit
editor-scripts = Site scripts
editor-scripts-help = Run the site's JavaScript in the canvas (off by default: clicks select elements)
editor-undo = Undo (Ctrl+Z)
editor-redo = Redo (Ctrl+Shift+Z)
editor-no-selection = No element selected
editor-owner-page = Page { $name }
editor-owner-layout = Layout { $name }
editor-owner-component = Component { $name }
layers-title = Layers
layers-up = Move up
layers-down = Move down
layers-wrap = Wrap in a container
layers-duplicate = Duplicate (Ctrl+D)
layers-delete = Delete (Del)
layers-if = if { $cond }
layers-else = else
layers-each = for { $item } in { $list }
layers-empty-list = if the list is empty
layers-image = image { $src }
layers-html = HTML
layers-var = variable { $name }
layers-slot = page content
layers-markdown = content (Markdown)
layers-badge-component = component
insert-title = Add
insert-where-selection = Added inside the selected element, or right after it.
insert-where-page = Added at the end of the page. Select an element to choose where.
insert-no-place = Select an element of the page where the new element should go.
insert-section = Section
insert-container = Container
insert-heading = Heading
insert-paragraph = Paragraph
insert-link = Link
insert-button = Button
insert-image = Image
insert-list = List
insert-text-heading = New heading
insert-text-paragraph = New paragraph.
insert-text-link = Link
insert-text-button = Button
insert-text-image = Image
insert-text-list = List item
inspector-title = Inspector
inspector-settings = Settings
inspector-none = Select an element in the page or in the layers.
inspector-tag = Tag
inspector-id = Identifier (id)
inspector-classes = Classes
inspector-class-add = Add a class
inspector-class-remove = Remove the class
inspector-link = Link
inspector-href = Address
inspector-new-tab = Open in a new tab
inspector-attributes = Other attributes
inspector-attr-add = Add: name=value
inspector-attr-remove = Remove the attribute
inspector-attr-boolean = (no value)
inspector-attr-when = Written only if: { $cond }
inspector-src = Image (address)
inspector-src-expr = Image (expression)
inspector-alt = Alternative text
inspector-alt-expr = Alternative text (expression)
inspector-condition = Condition
inspector-each-list = List
inspector-each-item = Name of each item
inspector-component = Component { $name }: its elements are set in the layers.
editor-bp-manage = Manage breakpoints
editor-bp-remove = Remove this breakpoint
editor-bp-add = Add
editor-bp-kind-max = Up to (narrower screens)
editor-bp-kind-min = From (wider screens)
editor-bp-in-use = This breakpoint has styles: remove them first in the styles panel.
styles-title = Styles
styles-selector = Selector
styles-state = State
styles-state-none = None
styles-state-hover = Hover (:hover)
styles-state-focus = Keyboard focus (:focus-visible)
styles-state-active = Pressed (:active)
styles-class-new = New class for this element
styles-no-class = This element has no class: add one to style it.
styles-where = Styles of { $selector } — { $breakpoint }
styles-new-rule = No rule here yet: the first value creates it, after wider screens.
styles-elements-only = Styles apply to HTML elements.
styles-reset = Remove the value
styles-important = Priority (!important)
styles-declarations = All declarations of the rule
styles-decl-add = Add: property: value
styles-layout = Layout
styles-spacing = Spacing
styles-size = Size
styles-position = Position
styles-typography = Typography
styles-backgrounds = Backgrounds
styles-borders = Borders
styles-effects = Effects
styles-display = Display
styles-direction = Direction
styles-justify = Justify
styles-align = Align
styles-gap = Gap
styles-columns = Columns (grid)
styles-margin = Margin
styles-padding = Padding
styles-width = Width
styles-height = Height
styles-min-width = Min width
styles-max-width = Max width
styles-min-height = Min height
styles-max-height = Max height
styles-overflow = Overflow
styles-position-type = Position
styles-top = Top
styles-right = Right
styles-bottom = Bottom
styles-left = Left
styles-z-index = Layer (z-index)
styles-font-family = Font
styles-font-size = Size
styles-font-weight = Weight
styles-line-height = Line height
styles-letter-spacing = Letter spacing
styles-color = Color
styles-text-align = Alignment
styles-text-transform = Case
styles-text-decoration = Decoration
styles-background-color = Color
styles-background-image = Image / gradient
styles-background-size = Size
styles-background-position = Position
styles-border-width = Width
styles-border-style = Style
styles-border-color = Color
styles-border-radius = Radius
styles-opacity = Opacity
styles-box-shadow = Shadow
styles-transform = Transform
styles-transition = Transition
styles-filter = Filter
nav-settings = Settings
home-col-name = Name
home-col-folder = Folder
home-col-opened = Last opened
settings-lang = Language
settings-theme = Theme
theme-system = System
theme-dark = Dark
theme-light = Light
settings-accent = Accent colour
settings-accent-help = Used for primary buttons, selection and focus. “None” keeps the interface neutral.
accent-none = None
accent-custom = Custom
overview-content = Content
overview-structure = Structure
overview-environments = Environments
action-close = Close
action-edit = Edit
action-replace = Replace
new-site-title = New site
new-site-choose = Choose folder
import-overwrite-title = Replace the existing site?
publish-default-name = My hosting
publish-form-new = New SFTP hosting
publish-form-edit = Edit hosting
publish-form-help = SFTP access provided by your host (OVH, o2switch, Infomaniak…), usually in your customer area under FTP-SSH.
publish-host-help = Example: ftp.cluster000.hosting.ovh.net
publish-remote-dir-help = The site's folder on the server, relative to your home folder. Example: www/
publish-password-help = Stored in the system keychain, never in the site files.
publish-check-help = Connects to the server and compares files, without sending or deleting anything.
publish-connecting = Connecting to the server…
publish-comparing = Comparing with the files online…
publish-check-failed = Check failed. { $message }
publish-delete-title = Delete “{ $name }”?
publish-delete-body = The saved access details and password will be erased. Files already online are not changed.
error-detail = { $detail }
error-no-desktop = This action requires the Revel desktop app.
error-project = The site could not be read: { $detail }
error-import = Import failed: { $detail }
error-io = Read or write error: { $detail }
error-already-imported = This folder already contains a Revel site.
error-not-eleventy = This folder is not an Eleventy project (no eleventy.config.js or .eleventy.js file).
error-unknown-profile = This hosting no longer exists. Reload the Publish page.
error-missing-secret = Password missing from the keychain: edit the hosting to enter it again.
error-keyring = The system keychain is not available: { $detail }
error-blocked = Publication blocked: fix the points listed first.
error-not-built = The site has not been generated for this environment yet. Generate it, then open the preview.
error-sftp-connect = Cannot connect to { $host }. Check the server address, the port and your internet connection. ({ $detail })
error-sftp-host-key = The server key has changed since the last connection. This may come from a change at your host or an interception attempt: check with your host before continuing.
error-sftp-auth = Login or password rejected by the server.
error-sftp = The server returned an error: { $detail }
error-sftp-remote-dir = Invalid folder on the server: “{ $dir }”. Enter a relative path, for example www/
error-sftp-file-name = Unsupported file name: { $name }
publish-fix = Edit hosting
publish-kind = Hosting type
publish-kind-sftp = SFTP server
publish-kind-github = GitHub Pages
publish-form-new-github = New GitHub Pages hosting
publish-form-help-github = GitHub Pages serves the site for free from a branch of a GitHub repository. Revel sends the generated site there and turns on GitHub Pages if needed.
publish-repo = Repository
publish-repo-help = Owner and repository name. Example: matv-studio/my-site
publish-branch = Branch
publish-branch-help = Branch reserved for the generated site, created if needed. Do not use the source branch.
publish-domain = Custom domain
publish-domain-help = Optional. Example: www.example.com (also declare it at your domain registrar). Empty: owner.github.io/repository/ address
publish-fetching = Fetching the online version…
publish-preparing = Preparing files: { $done } / { $total }
publish-open-site = View site
publish-pages-enabled = GitHub Pages has just been turned on for this repository: the site will be available within a few minutes.
github-account = GitHub account
github-connected = Connected to GitHub: { $login }
github-signed-out = No GitHub account connected.
github-connect = Connect to GitHub
github-disconnect = Disconnect
github-code-help = On github.com, enter this code to authorize Revel:
github-open = Copy code and open github.com
github-copy = Copy code
github-waiting = Waiting for your authorization on github.com… This updates automatically.
settings-accounts = Accounts
settings-github-help = Used to publish on GitHub Pages. Revel asks for access to your repositories; the token is kept in the system keychain.
error-github-signed-out = No GitHub account connected, or the connection has expired. Connect to GitHub, then try again.
error-github-repo = GitHub repository not found: “{ $repo }”. Check the name (owner/repository) and that the connected account can access it.
error-github-no-push = The connected GitHub account cannot write to the repository { $repo }.
error-github-cancelled = GitHub connection cancelled.
error-github-code-expired = The code expired before it was entered. Start the connection again.
error-github-denied = Authorization declined on github.com.
error-github-app = The Revel application is not set up correctly on GitHub ({ $detail }).
error-github-api = GitHub returned an error ({ $status }): { $detail }
error-git-rejected = The repository refused the push: { $detail }. The branch may have been changed elsewhere; publish again.
error-git = Git error: { $detail }
error-network = Cannot connect. Check your internet connection. ({ $detail })
error-edit-not-found = Element not found: it may have been deleted in the meantime. ({ $detail })
error-edit-invalid = This change is not possible here. ({ $detail })
error-edit-duplicate = Change refused: an element identifier would be duplicated. ({ $detail })
error-edit-move = An element cannot be placed inside itself.
error-edit-tag = Invalid tag name: letters, digits and hyphens only. ({ $detail })
error-edit-not-open = This site is not open in the editor.
error-edit-css = CSS refused: a single declaration, with no semicolon or brace outside quotes, and no “!important” in the value (separate checkbox). ({ $detail })
error-edit-no-stylesheet = This site does not have a stylesheet yet.
error-edit-breakpoints = Invalid breakpoints: each width can be used only once and must be a positive number of pixels. ({ $detail })
