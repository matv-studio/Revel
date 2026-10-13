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
deploy-warn-site-too-big = The site ({ $size } bytes) exceeds this host's limit ({ $max } bytes).
deploy-warn-file-too-big = A file ({ $size } bytes) exceeds this host's per-file limit ({ $max } bytes).
deploy-warn-too-many-files = The site has { $count } files; this host accepts at most { $max }.
deploy-warn-public-repo = On a free plan, this host requires the source repository to be public.
