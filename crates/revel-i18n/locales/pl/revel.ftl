app-title = Revel
app-tagline = Wizualny kreator witryn
app-version = Wersja { $version }
app-stage = Etap { $stage }: modułowy szkielet

menu-file = Plik
menu-site = Witryna
menu-publish = Publikacja
menu-help = Pomoc

language = Język

deploy-warn-no-301 = Ten hosting nie może wysyłać prawdziwych przekierowań 301: przekierowania (liczba: { $count }) staną się przekierowaniami po stronie klienta, które wyszukiwarki obsługują gorzej.
deploy-warn-no-php = Ten hosting nie uruchamia PHP: pliki PHP (liczba: { $count }), na przykład formularz kontaktowy, nie będą działać.
deploy-warn-no-headers = Ten hosting nie pozwala na własne nagłówki HTTP (indeksowanie, pamięć podręczna i bezpieczeństwo).
deploy-warn-no-htaccess = Ten hosting ignoruje pliki .htaccess: zawarte w nich reguły (HTTPS, przekierowania, dostęp) nie będą stosowane.
deploy-warn-site-too-big = Witryna ({ $size } MB) przekracza limit tego hostingu ({ $max } MB).
deploy-warn-file-too-big = Plik ({ $size } MB) przekracza limit rozmiaru pliku na tym hostingu ({ $max } MB).
deploy-warn-too-many-files = Liczba plików w witrynie: { $count }; ten hosting przyjmuje co najwyżej { $max }.
deploy-warn-public-repo = W planie darmowym ten hosting wymaga, aby repozytorium źródłowe było publiczne.

home-title = Witryny
home-empty = Brak witryn
home-drop = Upuść folder, aby dodać witrynę
gh-sync = Synchronizuj
gh-sync-help = Pobiera na ten komputer wersje Twoich witryn z GitHub i wysyła te zapisane tutaj; pokazuje witryny konta, których jeszcze tu nie ma
gh-sync-updated = Zaktualizowano: { $n }
gh-sync-sent = Wysłano: { $n }
gh-sync-waiting = Do sprawdzenia po otwarciu witryny: { $n }
gh-sync-failed = Niepowodzenia: { $n }
gh-sync-none = Wszystko jest aktualne
gh-site-download = Pobierz na ten komputer
gh-site-download-open = Pobierz i otwórz
gh-site-github = Otwórz w GitHub
action-open = Otwórz witrynę
action-import = Importuj
action-new = Nowa witryna
action-forget = Usuń z listy
action-cancel = Anuluj
action-save = Zapisz
action-delete = Usuń
new-site-name = Nazwa witryny
import-overwrite = Ten folder zawiera już witrynę Revel. Ponowny import zastąpi jej strukturę (strony, układy, kolekcje); treści i obrazy pozostaną bez zmian.
import-done = Import zakończony
import-warnings = Do sprawdzenia
site-pages = Strony
site-layouts = Układy
site-components = Komponenty
site-collections = Kolekcje
site-entries = { $count ->
        [one] { $count } wpis
        [few] { $count } wpisy
        [many] { $count } wpisów
       *[other] { $count } wpisu
    }
build-env = Środowisko
build-images = Obrazy responsywne (AVIF, WebP)
build-phase-pages = Strony
build-phase-copies = Pliki
build-phase-images = Obrazy { $done }/{ $total }
build-done = Wygenerowane strony: { $generated }, skopiowane pliki: { $copied }, obrazy: { $produced } nowych, { $cached } użytych ponownie, w { $seconds } s.
build-warnings = Ostrzeżenia
preview-component = Komponent · { $name }
preview-open = Podgląd
publish-empty = Ta witryna nie ma jeszcze hostingu. Dodaj serwer SFTP (OVH, o2switch…) lub GitHub Pages, aby publikować jednym kliknięciem.
publish-add = Dodaj hosting
publish-name = Nazwa
publish-env = Wygenerowane środowisko
publish-host = Serwer SFTP
publish-port = Port
publish-user = Login
publish-remote-dir = Folder na serwerze
publish-password = Hasło
publish-password-saved = Zapisane już w systemowym pęku kluczy. Zostaw puste, aby je zachować.
publish-check = Sprawdź
publish-run = Opublikuj
publish-planned = { $upload ->
        [one] { $upload } plik do wysłania
        [few] { $upload } pliki do wysłania
        [many] { $upload } plików do wysłania
       *[other] { $upload } pliku do wysłania
    } ({ $size } MB), do usunięcia: { $delete }, plików w witrynie: { $total }.
publish-progress = Wysyłanie: { $done } / { $total } { $total ->
        [one] plik
        [few] pliki
        [many] plików
       *[other] pliku
    }
publish-done = { $uploaded ->
        [0] { $deleted ->
            [0] Nic do opublikowania: witryna online jest już aktualna.
           *[other] Opublikowano: usunięto { $deleted } { $deleted ->
                [one] plik
                [few] pliki
                [many] plików
               *[other] pliku
            }.
        }
       *[other] Opublikowano: wysłano { $uploaded } { $uploaded ->
            [one] plik
            [few] pliki
            [many] plików
           *[other] pliku
        }, usunięto: { $deleted }.
    }
publish-dry-done = { $upload ->
        [0] { $delete ->
            [0] Sprawdzanie zakończone: witryna online jest już aktualna.
           *[other] Sprawdzanie zakończone, nic nie zmieniono na serwerze. Następna publikacja usunie { $delete } { $delete ->
                [one] plik
                [few] pliki
                [many] plików
               *[other] pliku
            }.
        }
       *[other] Sprawdzanie zakończone, nic nie zmieniono na serwerze. Następna publikacja wyśle { $upload } { $upload ->
            [one] plik
            [few] pliki
            [many] plików
           *[other] pliku
        } ({ $size } MB) i usunie: { $delete }.
    }
publish-key-recorded = Pierwsze połączenie: klucz serwera został zapisany i będzie wymagany przy kolejnych połączeniach. Porównaj jego odcisk z tym, który podaje Twój hosting:
publish-failed = Publikacja nie powiodła się. { $message }
publish-warnings = Przed publikacją
error = Błąd: { $message }
nav-recent = Ostatnie
nav-all-sites = Wszystkie witryny
nav-publish = Publikacja
# Visual editor (step 2)
editor-loading = Otwieranie witryny w edytorze…
editor-canvas = Kanwa strony
editor-page = Wyświetlana strona
editor-breakpoints = Punkty przerwania
editor-bp-desktop = Komputer (bazowy)
editor-bp-max = Do { $width } px
editor-bp-min = Od { $width } px
editor-zoom = Powiększenie
editor-undo = Cofnij (Ctrl+Z)
editor-redo = Ponów (Ctrl+Shift+Z)
editor-no-selection = Nie wybrano elementu
editor-bp-toggle = { $name }: pokaż lub ukryj na kanwie
editor-frame-height = Wysokość strony
editor-frame-height-help = Wyświetlana wysokość: { $height } px. Przeciągnij, aby wydłużyć lub skrócić; dwukrotne kliknięcie: wysokość domyślna
editor-side = Wszystkie ekrany
editor-dark = Podgląd w trybie ciemnym
editor-dark-help = Pokaż witrynę w kolorach trybu ciemnego, jak na urządzeniu z ciemnym motywem
editor-free = Swobodna kanwa
editor-free-help = Swobodna kanwa: przesuwaj widok, powiększaj, ustaw szerokość i wysokość ramki. Wyłączona: strona jest dopasowana do obszaru, a kółko myszy ją przewija
editor-side-help = Pokazuje wszystkie ekrany obok siebie; kliknij wewnątrz ekranu, aby go wybrać
palette = Polecenia
palette-placeholder = Szukaj akcji, elementu, strony…
palette-none = Brak wyników
palette-actions = Akcje
palette-view = Widok
palette-add = Dodaj
palette-goto = Przejdź do
palette-pages = Strony
palette-screens = Ekrany
palette-layers = Warstwy strony
keys-next = Następny element
keys-prev = Poprzedni element
keys-up = Przesuń element w górę
keys-down = Przesuń element w dół
keys-zoom-in = Powiększ
keys-zoom-out = Pomniejsz
keys-shift = Shift
keys-delete = Delete
keys-escape = Esc
keys-enter = Enter
keys-space = Spacja
prefs-editor = Edytor
prefs-editor-help = Powiązania między wariantami komponentów i rodzinami oferowanymi przez szybkie wyszukiwanie (Ctrl+K)
prefs-variants = Warianty komponentów
prefs-links = Powiązania
prefs-links-help = Kształt powiązań od wariantu głównego do wariantów wywoływanych gestem
prefs-links-curve = Zakrzywione
prefs-links-square = Pod kątem prostym
prefs-links-straight = Proste
prefs-link-end = Zakończenie
prefs-link-end-arrow = Strzałka
prefs-link-end-dot = Kropka
prefs-palette = Szybkie wyszukiwanie
prefs-palette-help = Rodziny wyników pokazywane przez szybkie wyszukiwanie (Ctrl+K)
prefs-colors = Kolory interfejsu
prefs-colors-help = Kod kolorów edytora, nigdy nie zastępowany kolorem akcentu: komponenty, wartość ustawiona tutaj, wartość odziedziczona, tłumaczenie maszynowe do sprawdzenia
prefs-color-comp = Komponenty
prefs-color-set = Ustawione tutaj
prefs-color-inherit = Odziedziczone
prefs-color-auto = Tłumaczenie maszynowe
prefs-color-reset = Kolor domyślny
prefs-shortcuts = Skróty
prefs-shortcuts-help = Klawisze każdej akcji edytora: kliknij Zmień, a następnie naciśnij wybraną kombinację (Esc anuluje)
prefs-key-press = Naciśnij klawisze…
prefs-key-none = Brak
prefs-key-change = Zmień
prefs-key-clear = Usuń skrót
prefs-key-reset = Klawisze domyślne
prefs-key-reset-all = Przywróć wszystko
prefs-key-conflict = Te same klawisze co: { $names }
key-palette = Ctrl+K · Ctrl+Space
editor-multi = Zaznaczone elementy: { $count }
editor-wrap-siblings = Aby je razem opakować, zaznaczone elementy muszą mieć tego samego rodzica.
editor-multi-add = Dodaj do zaznaczenia
key-multi = Shift+kliknięcie
editor-resize = Rozmiar
key-resize = Przeciągnij uchwyt
editor-owner-page = Strona { $name }
editor-owner-layout = Układ { $name }
editor-owner-component = Komponent { $name }
editor-saving = Zapisywanie…
editor-saved = Zapisano o { $time }
editor-saved-auto = Zapisano automatycznie
editor-shortcuts = Skróty klawiszowe
editor-undo-short = Cofnij
editor-redo-short = Ponów
layers-duplicate-short = Duplikuj element
layers-delete-short = Usuń element
editor-deselect = Odznacz
editor-edit-text = Edytuj tekst na stronie
editor-edit-validate = Zatwierdź tekst (Esc: anuluj)
key-undo = Ctrl+Z
key-redo = Ctrl+Shift+Z · Ctrl+Y
key-duplicate = Ctrl+D
key-delete = Delete
key-escape = Esc
key-dblclick = Dwukrotne kliknięcie
key-enter = Enter
layers-title = Warstwy
layers-up = Przesuń w górę
layers-down = Przesuń w dół
layers-wrap = Opakuj w kontener
layers-duplicate = Duplikuj
layers-delete = Usuń
layers-if = Warunek · { $cond }
layers-if-help = Wyświetlane zależnie od warunku ({ $cond }), w przeciwnym razie treść „Else”
layers-else = Else
layers-each = Lista · { $list }
layers-each-help = Powtarzane dla każdego elementu listy ({ $item } w { $list })
layers-condition = Wyświetlane pod warunkiem: { $cond }
layers-bound = Tekst z danych witryny lub wpisu CMS
layers-edit-text = Dwukrotne kliknięcie: edytuj tekst
layers-empty-list = jeśli lista jest pusta
layers-image = Obraz
layers-html = HTML
layers-slot = Treść strony
layers-markdown = Treść (Markdown)
layers-animated = Animowane (panel Interakcje)
layers-rename = Zmień nazwę
layers-hide = Ukryj
layers-show = Pokaż
key-hide = Ctrl+Shift+H
layers-hidden = Ukryte: nie ma go w opublikowanej witrynie
style-hide = Ukryj warstwę
style-show = Pokaż warstwę
el-heading = Nagłówek { $level }
el-section = Sekcja
el-div = Blok
el-p = Akapit
el-a = Link
el-button = Przycisk
el-img = Obraz
el-svg = Ikona
el-video = Wideo
el-audio = Dźwięk
el-iframe = Osadzenie
el-ul = Lista
el-ol = Lista numerowana
el-li = Element listy
el-nav = Nawigacja
el-header = Nagłówek strony
el-footer = Stopka
el-main = Treść główna
el-article = Artykuł
el-aside = Panel boczny
el-form = Formularz
el-input = Pole
el-textarea = Obszar tekstowy
el-select = Lista rozwijana
el-label = Etykieta
el-span = Span
el-strong = Pogrubienie
el-em = Kursywa
el-blockquote = Cytat
el-figure = Rysunek
el-figcaption = Podpis
el-table = Tabela
el-hr = Separator
el-body = Body
el-code = Kod
el-small = Mały tekst
el-time = Data
el-address = Adres
el-details = Pole rozwijane
el-summary = Tytuł pola rozwijanego
el-dialog = Okno dialogowe
el-other = Element
layers-badge-component = komponent
insert-title = Dodaj
insert-no-place = Wybierz element strony, w którym ma się znaleźć nowy element.
insert-section = Sekcja
insert-container = Kontener
insert-heading = Nagłówek
insert-paragraph = Akapit
insert-link = Link
insert-button = Przycisk
insert-image = Obraz
insert-list = Lista
insert-text-heading = Nowy nagłówek
insert-text-paragraph = Nowy akapit.
insert-text-link = Link
insert-text-button = Przycisk
insert-text-image = Obraz
insert-text-list = Element listy
inspector-title = Inspektor
inspector-settings = Ustawienia
inspector-advanced = Ustawienia zaawansowane
inspector-content-classes = Klasy zależne od treści
inspector-content-classes-help = Klasy dodawane według pól treści (wypełnione pole, wartość pola), każda z własnym stylem w Stylach
inspector-none = Wybierz element na stronie lub w warstwach.
inspector-tag = Tag
inspector-id = Identyfikator (id)
inspector-classes = Klasy
inspector-class-add = Dodaj klasę
inspector-class-remove = Usuń klasę
inspector-link = Link
inspector-href = Adres
inspector-heading-level = Poziom
link-section = Sekcja
link-section-help = Element strony, który ma identyfikator ID (ustawiany w ustawieniach zaawansowanych)
link-section-choose = Wybierz sekcję…
link-section-none = Brak sekcji (nadaj elementowi identyfikator ID)
link-file = Plik
link-kind = Typ linku
link-page = Strona
link-url = URL
link-email = E-mail
link-phone = Telefon
link-page-choose = Wybierz stronę…
inspector-new-tab = Otwórz w nowej karcie
inspector-current-section = Aktywny na podstronach
inspector-current-section-help = Link przyjmuje też stan „Bieżąca strona” na stronach, których adres zaczyna się od jego własnego (/projects/ dla /projects/my-project/). Na swojej własnej stronie przyjmuje go zawsze.
inspector-attributes = Inne atrybuty
inspector-attr-add = Dodaj: nazwa=wartość
inspector-attr-remove = Usuń atrybut
inspector-attr-boolean = (bez wartości)
attr-placeholder = Tekst zastępczy
attr-aria-label = Nazwa dla czytnika ekranu
attr-aria-hidden = Ukryte przed czytnikami ekranu
attr-aria-describedby = Opisane przez
attr-role = Rola (dostępność)
attr-title = Podpowiedź
attr-data-src = Źródło (ładowane później)
attr-src = Źródło
attr-srcset = Rozmiary obrazu
attr-loading = Ładowanie
attr-rel = Relacja linku
attr-download = Pobieranie
attr-tabindex = Kolejność klawiatury
attr-type = Typ
attr-name = Nazwa
attr-value = Wartość
attr-width = Szerokość
attr-height = Wysokość
attr-lang = Język
attr-datetime = Data (maszynowa)
attr-for = Powiązane pole
attr-action = Adres wysyłki
attr-method = Metoda wysyłki
attr-allow = Uprawnienia
attr-frameborder = Obramowanie ramki
attr-viewbox = Obszar rysunku
attr-decoding = Dekodowanie
attr-fetchpriority = Priorytet ładowania
inspector-attr-when = Zapisywane tylko jeśli: { $cond }
inspector-src = Obraz (adres)
inspector-src-expr = Obraz (wyrażenie)
inspector-alt = Tekst alternatywny
inspector-alt-expr = Tekst alternatywny (wyrażenie)
inspector-condition = Warunek
inspector-each-list = Lista
inspector-each-item = Nazwa każdego elementu
inspector-component = Komponent { $name }: jego elementy ustawia się w warstwach.
editor-bp-manage = Zarządzaj punktami przerwania
editor-bp-remove = Usuń ten punkt przerwania
editor-bp-add = Dodaj
editor-bp-kind-max = Do
editor-bp-kind-min = Od
editor-bp-in-use = Ten punkt przerwania ma style: najpierw usuń je w panelu stylów.
styles-title = Style
styles-selector = Klasa
styles-state = Stan
styles-state-none = Domyślny
styles-state-hover = Najechanie
styles-state-focus = Fokus klawiatury
styles-state-current = Bieżąca strona
styles-state-is-active = Aktywny
styles-state-in-active = Rodzic aktywny
styles-state-open = Otwarty
styles-state-in-open = Akordeon otwarty
styles-state-off = Wyłączony
styles-state-in-off = Przełącznik wyłączony
styles-state-active = Wciśnięty
styles-state-focus-any = Zaznaczony
styles-state-disabled = Wyłączony
styles-state-none-help = Style elementu w spoczynku.
styles-state-hover-help = Wskaźnik myszy nad elementem.
styles-state-active-help = Podczas naciskania lub klikania.
styles-state-focus-any-help = Pole lub przycisk zaznaczony myszą lub klawiaturą.
styles-state-disabled-help = Niedostępny przycisk lub pole (strzałka karuzeli na końcu).
styles-state-focus-help = Osiągnięty za pomocą klawiatury (klawisz Tab).
styles-state-current-help = Link do wyświetlanej strony (menu).
styles-state-is-active-help = Karta, filtr lub przełącznik, który jest włączony, element najechany w multimediach po najechaniu albo element wybrany losowo.
styles-state-in-active-help = Element wewnątrz aktywnego elementu (tekst aktywnej karty).
styles-state-open-help = Otwarty akordeon: kanwa otwiera go podczas edycji.
styles-state-in-open-help = Element wewnątrz otwartego akordeonu.
styles-state-off-help = Przełącznik dźwięku lub muzyki wyłączony.
styles-state-in-off-help = Element wewnątrz wyłączonego przełącznika.
styles-state-ready-help = Multimedia po najechaniu uruchomione (wideo) lub załadowane (obraz).
styles-state-in-paused-help = Element wewnątrz elementu zatrzymanego przez „Odtwórz lub wstrzymaj”.
styles-state-in-open-menu-help = Element w otwartym menu (pasek nawigacji na telefonie).
styles-state-in-empty-cart-help = Element w pustym koszyku (suma, przycisk zamówienia).
styles-class-new = Dodaj klasę…
styles-class-suggest = Sugerowane klasy
styles-class-create = Utwórz „{ $name }”
class-why-sibling = Używana przez sąsiedni element tego samego typu
class-why-family = Nazwana od klasy rodzica
class-why-neighbour = Używana przez sąsiedni element
class-why-tag = Używana gdzie indziej przez ten typ elementu
class-why-other = Inna klasa witryny
styles-tag-help = Stylizuje w witrynie każdy tag tego typu, bez klasy. Bez tego wyboru pierwsza ustawiona wartość nadaje elementowi klasę.
styles-tag-body = Body (wszystkie strony)
styles-tag-p = Wszystkie akapity
styles-tag-h1 = Wszystkie nagłówki H1
styles-tag-h2 = Wszystkie nagłówki H2
styles-tag-h3 = Wszystkie nagłówki H3
styles-tag-h4 = Wszystkie nagłówki H4
styles-tag-h5 = Wszystkie nagłówki H5
styles-tag-h6 = Wszystkie nagłówki H6
styles-tag-a = Wszystkie linki
styles-tag-img = Wszystkie obrazy
styles-tag-ul = Wszystkie listy punktowane
styles-tag-ol = Wszystkie listy numerowane
styles-tag-li = Wszystkie elementy listy
styles-tag-blockquote = Wszystkie cytaty blokowe
styles-tag-strong = Cały pogrubiony tekst
styles-tag-em = Cały tekst pochylony
styles-tag-figure = Wszystkie rysunki
styles-tag-section = Wszystkie sekcje
styles-tag-button = Wszystkie przyciski
styles-tag-video = Wszystkie filmy
styles-tag-svg = Wszystkie ikony
styles-tag-pre = Wszystkie bloki kodu
styles-tag-code = Cały kod
styles-tag-small = Cały mały tekst
styles-nested = { $tag } w { $parent }
styles-nested-help = Stylizuje wszystkie znaczniki tego typu w elementach z tą klasą: tekst sformatowany z CMS, wyróżnione słowa nagłówka
styles-nested-leave = Wróć do klasy
styles-nested-pick = Zawartość…
styles-nested-pick-help = Stylizuj znaczniki w tym elemencie (tekst sformatowany, wyróżnione słowa) przez jego klasę
styles-no-class = Ten element nie ma klasy: dodaj ją, aby go ostylować.
styles-new-rule = Nic tu jeszcze nie ustawiono: pierwsza wartość tworzy styl, stosowany na wierzchu szerszych ekranów.
styles-elements-only = Style dotyczą elementów HTML.
styles-reset = Resetuj
styles-important = Priorytet (!important)
styles-declarations = Kod CSS (zaawansowane)
styles-decl-add = Dodaj: właściwość: wartość
styles-layout = Układ
styles-spacing = Odstępy
styles-size = Rozmiar
styles-position = Pozycja
styles-typography = Tekst
styles-backgrounds = Tło
styles-borders = Obramowanie
styles-effects = Efekty
styles-display = Wyświetlanie
styles-direction = Kierunek
styles-align = Wyrównanie
styles-gap = Odstęp
styles-columns = Kolumny
styles-margin = Margines zewnętrzny
styles-padding = Margines wewnętrzny
styles-width = Szerokość
styles-height = Wysokość
styles-min-width = Min. szerokość
styles-max-width = Maks. szerokość
styles-min-height = Min. wysokość
styles-max-height = Maks. wysokość
styles-overflow = Przepełnienie
styles-position-type = Typ
styles-z-index = Kolejność warstw
text-field = Tekst
text-field-help = Tekst elementu; Enter zatwierdza, Shift+Enter zaczyna nowy wiersz. Można go też edytować, klikając dwukrotnie na stronie.
text-field-data = Tekst z danych witryny: zmieniany wszędzie, gdzie jest wyświetlany
text-field-entry = Tekst wpisu CMS: zmieniany w tym wpisie
text-field-prop = Właściwość tekstowa komponentu: zmieniana dla tej instancji
styles-font-family = Czcionka
styles-font-size = Rozmiar
styles-font-weight = Grubość
styles-line-height = Wysokość wiersza
styles-letter-spacing = Odstęp między literami
styles-color = Kolor
styles-fill = Wypełnienie (rysunek)
styles-text-align = Wyrównanie
styles-text-transform = Wielkość liter
styles-background-color = Kolor
styles-background-image = Obraz i gradient
styles-background-size = Rozmiar obrazu
styles-background-position = Pozycja
styles-border-width = Szerokość
styles-border-style = Styl linii
styles-border-color = Kolor
styles-outline = Obrys
styles-outline-style = Styl obrysu
styles-outline-color = Kolor obrysu
styles-outline-offset = Odstęp obrysu
styles-border-radius = Promień narożników
styles-opacity = Krycie
styles-box-shadow = Cienie
nav-settings = Ustawienia
home-col-name = Nazwa
home-col-folder = Folder
home-col-opened = Ostatnio otwarto
settings-lang = Język
settings-theme = Motyw
theme-system = Systemowy
theme-dark = Ciemny
theme-light = Jasny
settings-accent = Kolor akcentu
settings-accent-help = Kolor interfejsu; to, które elementy go przyjmują, ustawia się w „Zasięgu akcentu”. „Brak” zachowuje neutralny interfejs.
accent-none = Brak
accent-custom = Własny
settings-accent-scope = Zasięg akcentu
settings-accent-scope-help = Elementy interfejsu, które przyjmują kolor akcentu
settings-accent-scope-none = Najpierw wybierz kolor akcentu
accent-scope-minimal = Minimalny
accent-scope-minimal-help = Fokus klawiatury i obrys przestrzeni roboczej
accent-scope-standard = Standardowy
accent-scope-standard-help = Także przyciski główne, przełączniki i zaznaczenie
accent-scope-full = Rozszerzony
accent-scope-full-help = Także karty, wybory, aktywne wiersze, suwaki i paski postępu
overview-structure = Struktura
env-production = Produkcja
env-staging = Staging
site-facts-envs = Środowiska publikacji: { $list }
seo-org-add = Opisz organizację (nazwa, logo, dane kontaktowe), odczytywaną przez wyszukiwarki
seo-org-remove = Usuń opis organizacji
overview-environments = Środowiska
action-close = Zamknij
action-edit = Edytuj
action-replace = Zastąp
new-site-title = Nowa witryna
import-overwrite-title = Zastąpić istniejącą witrynę?
publish-default-name = Mój hosting
publish-form-new = Nowy hosting SFTP
publish-form-edit = Edytuj hosting
publish-form-help = Dostęp SFTP udostępniony przez Twój hosting (OVH, o2switch, Infomaniak…), zwykle w panelu klienta w sekcji FTP-SSH.
publish-host-help = Przykład: ftp.cluster000.hosting.ovh.net
publish-remote-dir-help = Folder witryny na serwerze, względem Twojego folderu domowego. Przykład: www/
publish-password-help = Przechowywane w systemowym pęku kluczy, nigdy w plikach witryny.
publish-check-help = Łączy się z serwerem i porównuje pliki, niczego nie wysyłając ani nie usuwając.
publish-connecting = Łączenie z serwerem…
publish-comparing = Porównywanie z plikami online…
publish-check-failed = Sprawdzanie nie powiodło się. { $message }
publish-delete-title = Usunąć „{ $name }”?
publish-delete-body = Zapisane dane dostępowe i hasło zostaną usunięte. Pliki już opublikowane online nie zmienią się.
error-detail = { $detail }
error-update = Brak dostępu do aktualizacji: { $detail }
error-no-desktop = Ta akcja wymaga aplikacji desktopowej Revel.
error-drop-folder-only = Upuść folder witryny, a nie pliki.
error-drop-folder-unknown = Nie znaleziono folderu „{ $name }”: wybierz go przez „Otwórz witrynę”.
error-site-thumb = Nie udało się zrobić obrazu strony głównej
error-icon-not-svg = Ten plik nie jest rysunkiem SVG.
error-project = Nie udało się odczytać witryny: { $detail }
error-import = Import nie powiódł się: { $detail }
error-io = Błąd odczytu lub zapisu: { $detail }
error-already-imported = Ten folder zawiera już witrynę Revel.
error-not-eleventy = Ten folder nie jest projektem Eleventy (brak pliku eleventy.config.js lub .eleventy.js).
error-unknown-profile = Ten hosting już nie istnieje. Odśwież stronę Publikacja.
error-missing-secret = Brak hasła w pęku kluczy: edytuj hosting, aby wprowadzić je ponownie.
error-keyring = Systemowy pęk kluczy jest niedostępny: { $detail }
error-blocked = Publikacja zablokowana: najpierw popraw wymienione punkty.
error-not-built = Witryna nie została jeszcze wygenerowana dla tego środowiska. Wygeneruj ją, a potem otwórz podgląd.
error-sftp-connect = Nie można połączyć się z { $host }. Sprawdź adres serwera, port i połączenie z internetem. ({ $detail })
error-sftp-timeout = Brak odpowiedzi od { $host }. Sprawdź adres serwera i port; w o2switch najpierw zezwól na adres IP tego komputera (cPanel › „Autorisation SSH”).
error-no-previous = Dla tego hostingu nie zachowano żadnej poprzedniej publikacji.
error-publish-checks = { $count ->
    [one] 1 pozycja do sprawdzenia przed publikacją.
    [few] { $count } pozycje do sprawdzenia przed publikacją.
    [many] { $count } pozycji do sprawdzenia przed publikacją.
   *[other] { $count } pozycji do sprawdzenia przed publikacją.
}
error-sftp-host-key = Klucz serwera zmienił się od ostatniego połączenia. Może to wynikać ze zmiany u Twojego dostawcy hostingu lub z próby przechwycenia połączenia: skonsultuj się z dostawcą, zanim przejdziesz dalej.
error-sftp-auth = Serwer odrzucił login lub hasło.
error-sftp = Serwer zwrócił błąd: { $detail }
error-sftp-remote-dir = Nieprawidłowy folder na serwerze: „{ $dir }”. Podaj ścieżkę względną, na przykład www/
error-sftp-file-name = Nieobsługiwana nazwa pliku: { $name }
publish-fix = Edytuj hosting
publish-kind = Typ hostingu
publish-kind-sftp = Serwer SFTP
publish-kind-github = GitHub Pages
publish-form-new-github = Nowy hosting GitHub Pages
publish-form-help-github = GitHub Pages udostępnia witrynę za darmo z gałęzi repozytorium GitHub. Revel wysyła tam wygenerowaną witrynę i w razie potrzeby włącza GitHub Pages.
publish-repo = Repozytorium
publish-repo-help = Właściciel i nazwa repozytorium. Przykład: matv-studio/my-site
publish-branch = Gałąź
publish-branch-help = Gałąź zarezerwowana dla wygenerowanej witryny, tworzona w razie potrzeby. Nie używaj gałęzi ze źródłami.
publish-domain = Własna domena
publish-domain-help = Opcjonalne. Przykład: www.example.com (zadeklaruj ją też u rejestratora domeny). Puste: adres owner.github.io/repository/
publish-fetching = Pobieranie wersji online…
publish-preparing = Przygotowywanie plików: { $done } / { $total }
publish-open-site = Zobacz witrynę
publish-pages-enabled = GitHub Pages zostało właśnie włączone dla tego repozytorium: witryna będzie dostępna w ciągu kilku minut.
github-account = Konto GitHub
github-connected = Połączono z GitHub: { $login }
github-signed-out = Brak połączonego konta GitHub.
github-connect = Połącz z GitHub
github-disconnect = Rozłącz
github-code-help = Na github.com wpisz ten kod, aby autoryzować Revel:
github-open = Skopiuj kod i otwórz github.com
github-copy = Kopiuj kod
github-waiting = Oczekiwanie na Twoją autoryzację na github.com… Ekran zaktualizuje się automatycznie.
settings-translate = Tłumaczenie automatyczne
settings-translate-help = Języki tłumaczone na tym komputerze, offline. Język jest pobierany jednorazowo z Hugging Face (modele OPUS-MT Uniwersytetu Helsińskiego); między dwoma językami tłumaczenie odbywa się przez angielski.
settings-translate-add = Dodaj
settings-translate-add-help = Dołączony do Revel: dodawany bez pobierania
settings-translate-get = Pobierz
settings-translate-get-help = Do pobrania jednorazowo: { $size }
settings-translate-remove = Usuń język
settings-translate-remove-title = Usunąć „{ $name }” z tłumaczenia automatycznego?
settings-translate-preparing = Przygotowywanie…
settings-appearance = Wygląd
settings-projects = Folder projektów
settings-projects-help = Nowe witryny są tworzone tutaj, każda we własnym folderze. Domyślnie: folder Projects obok Revel, jeśli można do niego zapisywać, w przeciwnym razie Documents\Revel.
settings-projects-reset = Wróć do folderu domyślnego
settings-accounts = Konta
settings-github-help = Służy do publikowania na GitHub Pages. Revel prosi o dostęp do Twoich repozytoriów; token jest przechowywany w systemowym pęku kluczy.
settings-author = Twoje imię i nazwisko
settings-author-help = Nazwa zapisywana w wersjach zapisanych z tego komputera, aby wiedzieć, kto co zmienił, gdy zespół pracuje razem. Bez nazwy: ta ustawiona w Git, w przeciwnym razie konto GitHub.
settings-author-placeholder = Imię Nazwisko
error-github-signed-out = Brak połączonego konta GitHub lub połączenie wygasło. Połącz z GitHub, a następnie spróbuj ponownie.
error-github-repo = Nie znaleziono repozytorium GitHub: „{ $repo }”. Sprawdź nazwę (właściciel/repozytorium) i czy połączone konto ma do niego dostęp.
error-github-no-push = Połączone konto GitHub nie może zapisywać w repozytorium { $repo }.
error-github-cancelled = Połączenie z GitHub zostało anulowane.
error-github-code-expired = Kod wygasł, zanim został wpisany. Rozpocznij połączenie od nowa.
error-github-denied = Autoryzacja odrzucona na github.com.
error-github-app = Aplikacja Revel nie jest poprawnie skonfigurowana w GitHub ({ $detail }).
error-github-api = GitHub zwrócił błąd ({ $status }): { $detail }
error-git-rejected = Repozytorium odrzuciło wysłanie zmian: { $detail }. Gałąź mogła zostać zmieniona gdzie indziej; opublikuj ponownie.
error-git = Błąd Git: { $detail }
error-network = Nie można się połączyć. Sprawdź połączenie z internetem. ({ $detail })
error-edit-not-found = Nie znaleziono elementu: mógł zostać w międzyczasie usunięty. ({ $detail })
error-edit-invalid = Ta zmiana nie jest tu możliwa. ({ $detail })
error-edit-duplicate = Zmiana odrzucona: identyfikator elementu zostałby zduplikowany. ({ $detail })
error-edit-move = Element nie może zostać umieszczony wewnątrz samego siebie.
error-edit-tag = Nieprawidłowa nazwa tagu: tylko litery, cyfry i myślniki. ({ $detail })
error-edit-not-open = Ta witryna nie jest otwarta w edytorze.
error-font = Czcionka: { $detail }
error-edit-site = Nieprawidłowy adres: „{ $detail }” (oczekiwano: https://…)
error-edit-class = Nazwa klasy odrzucona: litery, cyfry i myślniki, nie może zaczynać się od cyfry.
error-edit-class-taken = Ta klasa już istnieje.
sync-title = Praca zespołowa
sync-ok = Aktualne · sprawdzono o { $time }
sync-busy = Odbieranie wersji…
sync-off = Synchronizacja automatyczna jest wyłączona (Ustawienia › Wersje)
sync-offline = Brak odpowiedzi od wspólnego repozytorium: ponowna próba za minutę
sync-waiting = Wersje powstały po obu stronach: zostaną połączone przy następnym zapisie wersji
sync-paused = { $count ->
    [one] Synchronizacja nie mogła się zakończyć: { $count } miejsce zmieniono po obu stronach. Sprawdź, którą wersję zachować, lub odłóż.
    [few] Synchronizacja nie mogła się zakończyć: { $count } miejsca zmieniono po obu stronach. Sprawdź, którą wersję zachować, lub odłóż.
    [many] Synchronizacja nie mogła się zakończyć: { $count } miejsc zmieniono po obu stronach. Sprawdź, którą wersję zachować, lub odłóż.
   *[other] Synchronizacja nie mogła się zakończyć: { $count } miejsca zmieniono po obu stronach. Sprawdź, którą wersję zachować, lub odłóż.
}
sync-received = { $count ->
    [one] Otrzymano wersję od: { $authors }
    [few] Otrzymane wersje ({ $count }) od: { $authors }
    [many] Otrzymane wersje ({ $count }) od: { $authors }
   *[other] Otrzymane wersje ({ $count }) od: { $authors }
}
sync-last-received = { $count ->
    [one] Ostatnia wersja otrzymana od: { $authors } o { $time }
    [few] Otrzymane wersje ({ $count }) od: { $authors } o { $time }
    [many] Otrzymane wersje ({ $count }) od: { $authors } o { $time }
   *[other] Otrzymane wersje ({ $count }) od: { $authors } o { $time }
}
sync-verify = Sprawdź
sync-later = Później
sync-now = Synchronizuj
versions-auto = Synchronizacja automatyczna
versions-auto-help = Wersje innych osób docierają same (bez tworzenia wersji, przy pierwszej przerwie); Twoje są wysyłane, gdy tylko zapiszesz wersję
error-edit-outdated = Tej zmiany nie można już cofnąć: współpracownik zmienił to samo miejsce
error-translate-pair = Brak modelu tłumaczenia dla { $from } → { $to }.
error-translate = Tłumaczenie automatyczne nie powiodło się: { $detail }
error-versions-conflict = Te same wiersze zmieniono online i w witrynie: rozwiąż konflikt w Ustawienia › Wersje przed publikacją.
error-translate-integrity = Pobrany plik tłumaczenia jest uszkodzony ({ $detail }): pobierz go ponownie.
error-translate-cancelled = Pobieranie anulowane.
error-translate-busy = Trwa już pobieranie języka.
error-edit-css = CSS odrzucony: pojedyncza deklaracja, bez średnika ani nawiasu klamrowego poza cudzysłowami i bez „!important” w wartości (osobne pole wyboru). ({ $detail })
error-media = Nie zaimportowano multimediów: { $detail }
error-edit-expr = Nieprawidłowe wyrażenie, nic nie zostało zmienione. ({ $detail })
error-edit-path = Adres odrzucony: zaczyna się od „/” i zawiera tylko małe litery bez znaków diakrytycznych, cyfry i myślniki. ({ $detail })
error-edit-path-taken = Ten adres jest już używany przez inną stronę. ({ $detail })
error-edit-redirect = Przekierowanie odrzucone: stary adres zaczyna się od „/” i różni się od nowego. ({ $detail })
error-edit-in-use = Ten komponent jest nadal używany: najpierw odłącz lub usuń jego instancje. ({ $detail })
error-edit-prop = Nazwa właściwości odrzucona: litery bez znaków diakrytycznych, cyfry i „_”, jedna na komponent. ({ $detail })
error-edit-self-use = Komponent nie może zostać umieszczony wewnątrz samego siebie. ({ $detail })
error-edit-locale = Tego języka nie można dodać lub nie istnieje (kod: kod języka, taki jak en lub pt-BR, inny niż język główny). ({ $detail })
error-edit-no-stylesheet = Ta witryna nie ma jeszcze arkusza stylów.
error-edit-breakpoints = Nieprawidłowe punkty przerwania: każda szerokość może być użyta tylko raz i musi być dodatnią liczbą pikseli. ({ $detail })
error-edit-bound = Ten tekst pochodzi z pliku, którego nie można tu zmieniać. ({ $detail })
error-mcp-no-site = W Revel nie jest otwarta żadna witryna: otwórz jedną lub podaj folder witryny.
error-mcp-disabled = Serwer asystenta AI jest wyłączony: włącz go w Revel, Ustawienia › Asystenci AI.
error-mcp-unreachable = Revel nie odpowiedział: otwórz Revel i sprawdź, czy serwer asystenta AI jest włączony.
error-mcp-token = Revel odrzucił klucz dostępu: skopiuj aktualny klucz z Ustawienia › Asystenci AI.

## Panneau de styles (D-20)
styles-count = { $count ->
        [one] { $count } element
        [few] { $count } elementy
        [many] { $count } elementów
       *[other] { $count } elementu
    }
styles-origin-here = Ustawione tutaj: { $source }
styles-origin-inherited = Odziedziczone z: { $source }
styles-origin-none = Wartość pochodząca skądinąd (inna klasa, element nadrzędny lub przeglądarka).
styles-label-help = Przeciągnij: zmień wartość (Shift ×10, Alt ×0,1). Alt+kliknięcie: resetuj
styles-unset = –
display-block = Blok
display-flex = Flex
display-grid = Grid
display-none = Ukryty
display-more = Inne tryby wyświetlania
display-inline = W linii
display-inline-block = Blok w linii
display-inline-flex = Flex w linii
display-inline-grid = Grid w linii
display-contents = Tylko zawartość
dir-horizontal = Poziomo
dir-vertical = Pionowo
dir-reverse = Odwrócona kolejność
align-horizontal = Poziomo
align-vertical = Pionowo
align-cell = Wyrównanie: kolumna { $col }, wiersz { $row }
align-left = Do lewej
align-center = Do środka
align-right = Do prawej
align-top = Do góry
align-bottom = Do dołu
align-stretch = Rozciągnij
align-baseline = Linia bazowa
align-between = Odstęp między
align-around = Odstęp wokół
align-evenly = Równy odstęp
styles-wrap = Zawijanie
styles-flex-child = Element potomny flex
flex-shrink = Zmniejszaj w razie potrzeby
flex-grow = Powiększaj, jeśli to możliwe
flex-none = Nie zmniejszaj ani nie powiększaj
styles-spacing-help = Przeciągnij wartość, aby ją zmienić (Alt: także przeciwna strona, Shift: wszystkie cztery strony), kliknij, aby ją wpisać.
styles-side-help = przeciągnij lub kliknij
overflow-visible = Widoczne
overflow-hidden = Ukryte
overflow-scroll = Przewijanie
overflow-auto = Auto
styles-ratio = Proporcje
ratio-auto = Dowolne
ratio-square = Kwadrat (1:1)
ratio-wide = Szeroki (16:9)
ratio-classic = Klasyczny (4:3)
ratio-landscape = Poziomy (3:2)
ratio-portrait = Pionowy (2:3)
ratio-tall = Wysoki (9:16)
styles-fit = Dopasowanie
styles-object-position = Kadrowanie
fit-cover = Wypełnij (przytnij)
fit-contain = Dopasuj (pokaż całość)
fit-fill = Rozciągnij
fit-none = Rozmiar rzeczywisty
fit-scale-down = Zmniejsz w razie potrzeby
unit-menu = Jednostka
nbox-fluid = Płynna
nbox-fluid-help = Wartość płynna: między minimum a maksimum, zależnie od szerokości ekranu
nbox-fn-min = Min
nbox-fn-ideal = Idealna
nbox-fn-max = Maks.
nbox-fn-value = Wartość { $n }
nbox-fixed = Wartość stała
nbox-fixed-help = Zastąp pojedynczą wartością
nbox-vars = Zmienne witryny
nbox-detach = Odłącz od zmiennej
nbox-detach-help = Zastąp bieżącą wartością zmiennej
unit-px = piksele, rozmiar stały
unit-percent = procent elementu nadrzędnego
unit-vw = procent szerokości ekranu
unit-vh = procent wysokości ekranu
unit-fit-screen = rozmiar tekstu zajmujący tę część wysokości ekranu, bez wychodzenia poza szerokość
unit-em = względem rozmiaru tekstu
unit-rem = względem rozmiaru tekstu strony
unit-auto = automatycznie
unit-fit = dopasuj do zawartości
unit-none = bez limitu
unit-normal = normalnie
unit-times = razy rozmiar tekstu
position-static = Statyczna
position-relative = Względna
position-absolute = Bezwzględna
position-fixed = Stała
position-sticky = Przyklejona
position-static-help = Podąża za przepływem strony.
position-relative-help = Przesunięta względem swojego normalnego miejsca; stanowi punkt odniesienia dla elementów o pozycji bezwzględnej wewnątrz niej.
position-absolute-help = Umieszczona względem najbliższego pozycjonowanego elementu nadrzędnego, poza przepływem strony.
position-fixed-help = Umieszczona względem okna: pozostaje widoczna podczas przewijania.
position-sticky-help = Przewija się normalnie, a następnie przykleja do wskazanej krawędzi.
styles-offsets = Przesunięcia
styles-preset = Ustawienie wstępne
preset-choose = Wybierz…
preset-full = Cały element nadrzędny
preset-top = Górna krawędź
preset-bottom = Dolna krawędź
preset-left = Lewa krawędź
preset-right = Prawa krawędź
preset-top-left = Lewy górny róg
preset-top-right = Prawy górny róg
preset-bottom-left = Lewy dolny róg
preset-bottom-right = Prawy dolny róg
font-system = Czcionka systemowa
font-serif = Szeryfowa
font-mono = Stałej szerokości
weight-variable = Zmienna
weight-not-loaded = { $name } (niezaładowana)
pos-top-left = Lewy górny
pos-top = Góra
pos-top-right = Prawy górny
pos-left = Lewo
pos-center = Środek
pos-right = Prawo
pos-bottom-left = Lewy dolny
pos-bottom = Dół
pos-bottom-right = Prawy dolny
weight-100 = Bardzo cienka
weight-200 = Ultralekka
weight-300 = Lekka
weight-400 = Zwykła
weight-500 = Średnia
weight-600 = Półgruba
weight-700 = Gruba
weight-800 = Bardzo gruba
weight-900 = Czarna
text-left = Do lewej
text-center = Do środka
text-right = Do prawej
text-justify = Wyjustuj
styles-text-style = Styl
text-italic = Kursywa
text-underline = Podkreślenie
text-strike = Przekreślenie
case-none = Jak wpisano
case-upper = Wielkie litery
case-lower = Małe litery
case-capitalize = Każde słowo wielką literą
list-none = Brak
list-disc = Kropka
list-circle = Okrąg
list-square = Kwadrat
list-decimal = Numery
bg-add = Dodaj obraz lub gradient
bg-none = Brak
bg-image = Obraz
bg-linear = Gradient liniowy
bg-radial = Gradient promieniowy
bg-remove = Usuń tę warstwę
bg-image-change = Zmień
bg-angle = Kąt
bg-angle-help = Przeciągnij, aby obrócić gradient (Shift: co 15°)
bg-shape = Kształt
bg-circle = Koło
bg-ellipse = Elipsa
bg-center = Środek
bg-center-x = Środek w poziomie
bg-center-y = Środek w pionie
bg-stops = Kolory
bg-stops-help = Przeciągnij kolor, aby go przesunąć, kliknij pasek, aby dodać kolejny
bg-stop = Kolor gradientu
bg-stop-color = Kolor
bg-stop-position = Położenie koloru
bg-stop-remove = Usuń ten kolor
bg-cover = Wypełnij
bg-contain = Dopasuj
bg-auto = Rozmiar rzeczywisty
styles-background-repeat = Powtarzanie
corner-independent = Niezależne narożniki
corner-top-left = Lewy górny
corner-top-right = Prawy górny
corner-bottom-left = Lewy dolny
corner-bottom-right = Prawy dolny
styles-border-sides = Strony
side-all = Wszystkie strony
side-top = Góra
side-right = Prawo
side-bottom = Dół
side-left = Lewo
border-none = Brak
border-solid = Ciągła
border-dashed = Kreskowana
border-dotted = Kropkowana
shadow-none = Bez cienia
shadow-inherited = Cień odziedziczony
shadow-add = Dodaj cień
shadow-remove = Usuń cień
shadow-type = Typ cienia
shadow-drop = Cień zewnętrzny
shadow-inner = Cień wewnętrzny
shadow-x = Przesunięcie w poziomie
shadow-y = Przesunięcie w pionie
shadow-blur = Rozmycie
shadow-spread = Rozpiętość
shadow-color = Kolor cienia
styles-cursor = Kursor
styles-pointer-events = Kliknięcia
pointer-events-auto = Odbierane
pointer-events-none = Przechodzą
cursor-auto = Automatyczny
cursor-default = Strzałka
cursor-pointer = Dłoń (link)
cursor-text = Tekst
cursor-move = Przesuwanie
cursor-grab = Chwytanie
cursor-not-allowed = Niedozwolone
cursor-help = Pomoc
cursor-none = Ukryty
color-open = Wybierz kolor
color-plane = Nasycenie i jasność
color-hue = Odcień
color-opacity = Krycie
color-eyedropper = Pobierz kolor z ekranu
color-model = Model kolorów
color-rgb = RGB
color-hsl = HSL
color-hsb = HSB
color-none = Brak
color-site = Kolory witryny
color-site-add = Dodaj do kolorów witryny
color-site-name = Nazwa koloru
color-site-default = Kolor { $n }
color-site-create = Dodaj
colors-title = Kolory
colors-help = Kolory witryny, używane ponownie w stylach. Przy trybie ciemnym każdy kolor ma też wartość dla odwiedzających, których urządzenie używa ciemnego motywu.
sizes-title = Rozmiary
sizes-help = Rozmiary witryny (tekst, odstępy, szerokości), używane przez style jak kolory. Rozmiar płynny rośnie z szerokością ekranu między minimum a maksimum. Zmiana nazwy aktualizuje wszystkie style, które go używają.
sizes-add = Dodaj rozmiar
sizes-name = Nazwa rozmiaru
sizes-default = Tekst L
sizes-value = Wartość
sizes-fluid-help = Rozmiar płynny: od minimum na telefonie do maksimum na dużym ekranie
sizes-min = Minimum
sizes-max = Maksimum
sizes-rename = Zmień nazwę (wszystkie style, które jej używają, zostaną zaktualizowane)
sizes-uses-help = Liczba stylów, które jej używają
sizes-none = Brak rozmiarów
colors-name = Nazwa
colors-light = Jasny
colors-dark = Ciemny
colors-none = Brak kolorów witryny
colors-dark-mode = Tryb ciemny
colors-dark-help = Witryna podąża za jasnym lub ciemnym motywem urządzenia odwiedzającego
colors-dark-off-title = Usunąć kolory trybu ciemnego?
colors-remove-title = Usunąć kolor „{ $name }”?
colors-remove-used = Użyto w stylach: { $count }. Te style stracą swój kolor.
styles-contrast = Kontrast z tłem: { $ratio }:1 · { $level }
styles-contrast-aaa = AAA
styles-contrast-aa = AA
styles-contrast-large = AA tylko dla dużego tekstu
styles-contrast-low = zbyt niski
unit-auto-s = Auto
unit-fit-s = Dopasuj
unit-none-s = Brak
unit-normal-s = Normalnie
unit-fit-screen-s = Ekran
styles-label-help-reset = Alt+kliknięcie: resetuj
styles-margin-short = Margines zewn.
styles-padding-short = Margines wewn.
styles-combo = Klasy łączone: styl dotyczy tylko elementów, które mają je wszystkie
styles-class-remove = Usuń tę klasę z elementu
styles-class-first = Dodaj klasę…
styles-new = Nowy
editor-bp-desktop-short = Komputer

## Espace de travail (D-21)
panel-pages = Strony
panel-cms = CMS
panel-components = Komponenty
panel-assets = Zasoby
panel-classes = Klasy
classes-search = Szukaj klas
classes-count = { $count ->
    [one] 1 klasa
    [few] { $count } klasy
    [many] { $count } klas
   *[other] { $count } klasy
}
classes-none = Brak klas
classes-no-match = Nie znaleziono klasy.
classes-unused = nieużywana
classes-unused-note = Żaden element nie ma tej klasy
classes-unused-filter = Tylko nieużywane klasy
classes-flatten = Udostępnij do edycji w panelu Style ({ $count } ustawień zapisanych jako kod)
classes-flatten-title = Udostępnić do edycji style zapisane jako kod?
classes-flatten-body = { $moved } reguł przeniesionych na klasy elementów (dodano { $classes } klas, { $pseudo } elementów), usunięto { $dead } nieużywanych reguł, { $kept } zachowano bez zmian.
classes-flatten-apply = Udostępnij do edycji
classes-flatten-none = Brak reguł do przeniesienia
classes-clean = { $count ->
    [0] Brak nieużywanych klas
    [one] Usuń nieużywaną klasę
    [few] Usuń { $count } nieużywane klasy
    [many] Usuń { $count } nieużywanych klas
   *[other] Usuń { $count } nieużywanej klasy
}
classes-clean-title = { $count ->
    [one] Usunąć nieużywaną klasę?
    [few] Usunąć { $count } nieużywane klasy?
    [many] Usunąć { $count } nieużywanych klas?
   *[other] Usunąć { $count } nieużywanej klasy?
}
classes-row-title = { $uses ->
    [0] Brak elementów
    [one] 1 element
    [few] { $uses } elementy
    [many] { $uses } elementów
   *[other] { $uses } elementu
} · { $count ->
    [0] brak stylów
    [one] 1 styl
    [few] { $count } style
    [many] { $count } stylów
   *[other] { $count } stylu
}
classes-go = Przejdź do elementu
classes-in-layout = Szablon { $name }
classes-rename = Zmień nazwę
classes-rename-title = Zmień nazwę „{ $name }”
classes-rename-label = Nowa nazwa (wszędzie: style, elementy, interakcje)
classes-remove = Usuń
classes-remove-title = Usunąć klasę „{ $name }”?
classes-remove-body = { $uses ->
    [one] Usunięto z 1 elementu
   *[other] Usunięto z { $uses } elementów
}; { $count ->
    [0] brak stylów do usunięcia.
    [one] usunięto 1 styl.
    [few] usunięto { $count } style.
    [many] usunięto { $count } stylów.
   *[other] usunięto { $count } stylu.
}
classes-remove-unused-body = { $count ->
    [0] Żaden element jej nie ma i nie ma ona stylów.
    [one] Żaden element jej nie ma; usunięto 1 styl.
    [few] Żaden element jej nie ma; usunięto { $count } style.
    [many] Żaden element jej nie ma; usunięto { $count } stylów.
   *[other] Żaden element jej nie ma; usunięto { $count } stylu.
}
styles-class-title = { $name }: kliknij dwukrotnie, aby zmienić nazwę wszędzie
panel-interactions = Interakcje
anim-appear = Pojawianie się
anim-appear-help = Gdy element wchodzi w widok lub przy ładowaniu strony
anim-text = Tekst
anim-text-help = Litery, słowa lub wiersze animowane jedno po drugim
anim-hover = Najechanie
anim-hover-help = Gdy wskaźnik jest nad elementem (komputery)
anim-press = Naciśnięcie
anim-press-help = Podczas klikania lub dotykania
anim-loop = Pętla
anim-loop-help = W sposób ciągły, wstrzymywana poza ekranem
anim-scroll = Przewijanie
anim-scroll-help = Podąża za położeniem elementu podczas przewijania strony
anim-active = Staje się aktywny
anim-active-help = Za każdym razem, gdy element lub karta, filtr albo przełącznik, który go zawiera, staje się aktywny
anim-add = Dodaj animację: { $name }
anim-remove = Usuń animację
anim-preview = Podgląd na kanwie
anim-wrap = Opakuj, aby animować
anim-wrap-help = Obraz jest animowany przez zawierający go blok: zostaje utworzony wokół niego i zaznaczony
anim-p-fade = Przenikanie
anim-p-up = Wznoszenie
anim-p-down = Opadanie
anim-p-left = Z lewej
anim-p-right = Z prawej
anim-p-zoom-in = Przybliżenie
anim-p-zoom-out = Oddalenie
anim-p-blur = Rozmycie
anim-p-pop = Wyskok
anim-p-flip = Przewrót
anim-p-rotate = Obrót
anim-p-reveal = Odsłonięcie
anim-p-typewriter = Maszyna do pisania
anim-p-grow = Powiększenie
anim-p-shrink = Zmniejszenie
anim-p-lift = Uniesienie
anim-p-shadow = Cień
anim-p-dim = Przyciemnienie
anim-p-tilt = Przechył
anim-p-brighten = Rozjaśnienie
anim-p-underline = Podkreślenie
anim-p-press = Wciśnięcie
anim-p-squish = Ściśnięcie
anim-p-push = Pchnięcie
anim-p-spin = Wirowanie
anim-p-pulse = Pulsowanie
anim-p-float = Unoszenie
anim-p-swing = Kołysanie
anim-p-shake = Trzęsienie
anim-p-blink = Mruganie
anim-p-bounce = Odbijanie
anim-p-heartbeat = Bicie serca
anim-p-ticker = Taśma przewijana
anim-p-parallax = Paralaksa
anim-p-fade-in = Pojawienie się
anim-p-zoom-in-scroll = Przybliżenie
anim-p-rotate-scroll = Rotacja
anim-p-slide-x = Przesunięcie
anim-p-fade-out = Wyjście
anim-p-jump = Skok
anim-p-flash = Błysk
anim-d-appear-fade = Krycie od 0 do 100%
anim-d-appear-up = Wchodzi od dołu
anim-d-appear-down = Wchodzi od góry
anim-d-appear-left = Wchodzi z lewej
anim-d-appear-right = Wchodzi z prawej
anim-d-appear-zoom-in = Rośnie z 90% do 100%
anim-d-appear-zoom-out = Maleje z 110% do 100%
anim-d-appear-blur = Nabiera ostrości (rozmycie 12 px)
anim-d-appear-pop = Rośnie z 50% do 100% z lekkim przeregulowaniem
anim-d-appear-flip = Obraca się do przodu w 3D
anim-d-appear-rotate = Lekko się obraca, unosząc się
anim-d-appear-reveal = Odsłaniany od dołu do góry, jak kurtyna
anim-d-text-fade = Każdy fragment się pojawia
anim-d-text-up = Każdy fragment unosi się, pojawiając się
anim-d-text-blur = Każdy fragment nabiera ostrości
anim-d-text-pop = Każdy fragment rośnie z przeregulowaniem
anim-d-text-flip = Każdy fragment obraca się w 3D
anim-d-text-typewriter = Litery pokazywane jedna po drugiej
anim-d-hover-grow = Skala 105%
anim-d-hover-shrink = Skala 95%
anim-d-hover-lift = Unosi się o 6 px z cieniem
anim-d-hover-shadow = Mocniejszy cień zewnętrzny
anim-d-hover-dim = Krycie 70%
anim-d-hover-tilt = Obrót o 3°
anim-d-hover-brighten = Jasność 115%
anim-d-hover-underline = Linia rysowana pod tekstem
anim-d-press-press = Skala 95%
anim-d-press-squish = Ściskany, a potem puszczany
anim-d-press-push = Przesuwa się w dół o 2 px
anim-d-loop-spin = Równomierny pełny obrót
anim-d-loop-pulse = Rośnie do 106% i wraca
anim-d-loop-float = Przesuwa się w górę i w dół o 10 px
anim-d-loop-swing = Kołysze się o ±6°
anim-d-loop-shake = Trzęsie się w lewo i w prawo
anim-d-loop-blink = Krycie od 100% do 30%
anim-d-loop-bounce = Podskakuje i ląduje
anim-d-loop-heartbeat = Podwójne bicie serca
anim-d-loop-ticker = Zawartość przewijana bez końca (logo, słowa)
anim-d-scroll-parallax = Porusza się wolniej niż strona (80 px)
anim-d-scroll-fade-in = Pojawia się, wchodząc do okna
anim-d-scroll-zoom-in = Rośnie z 85% do 100% przy wejściu
anim-d-scroll-rotate = Obraca się podczas przechodzenia przez okno (±10°)
anim-d-scroll-slide-x = Przechodzi przez okno z prawej do lewej (120 px)
anim-d-scroll-fade-out = Zanika, wychodząc górą
anim-d-active-pop = Rośnie do 150% i wraca
anim-d-active-jump = Podskakuje o 12 px i ląduje
anim-d-active-shake = Trzęsie się w lewo i w prawo
anim-d-active-flash = Krycie od 100% do 30% i z powrotem
anim-d-active-spin = Wykonuje pełny obrót
anim-trigger = Wyzwalacz
anim-trigger-view = W widoku
anim-trigger-load = Ładowanie strony
anim-trigger-view-help = Gdy element wchodzi w widok
anim-trigger-load-help = Zaraz po załadowaniu strony
anim-dur = Czas trwania
anim-dur-help = Czas trwania animacji w sekundach
anim-dur-loop-help = Czas trwania jednego cyklu w sekundach
anim-dur-text-help = Czas trwania dla każdego fragmentu w sekundach
anim-delay = Opóźnienie
anim-delay-help = Czas oczekiwania przed startem w sekundach
anim-ease = Krzywa
anim-ease-smooth = Gładka
anim-ease-bounce = Odbicie
anim-ease-ease = Równa
anim-ease-linear = Liniowa
anim-dist = Odległość
anim-dist-help = Odległość pokonywana przy pojawianiu się, w pikselach
anim-stagger = Kaskada
anim-stagger-help = Zawarte elementy animowane jeden po drugim: czas między dwoma w sekundach (0: cały element)
anim-stagger-text-help = Czas między dwoma fragmentami w sekundach
anim-replay = Powtarzanie
anim-replay-help = Odtwarzane ponownie za każdym razem, gdy wchodzi w widok
anim-split = Podział
anim-split-chars = Litery
anim-split-words = Słowa
anim-split-lines = Wiersze
anim-amount = Intensywność
anim-amount-help = Siła efektu jako procent zamierzonej wartości
anim-mirror = Tam i z powrotem
anim-mirror-help = Każdy cykl wraca zamiast zaczynać od nowa
anim-dir = Kierunek
anim-dir-left = W lewo
anim-dir-right = W prawo
anim-pause = Wstrzymaj po najechaniu
panel-variables = Zmienne
variables-search = Szukaj zmiennych
variables-none = Brak zmiennych na tej stronie
variables-no-match = Brak pasującej zmiennej
variables-owner-layout = Układ { $name }
variables-owner-component = Komponent { $name }
variables-owner-page = Strona { $name }
variables-value-empty = puste
variables-value-list = { $count ->
        [0] pusta lista
        [one] lista · 1 element
        [few] lista · { $count } elementy
        [many] lista · { $count } elementów
       *[other] lista · { $count } elementu
    }
variables-value-object = { $count ->
        [0] pusty obiekt
        [one] obiekt · 1 klucz
        [few] obiekt · { $count } klucze
        [many] obiekt · { $count } kluczy
       *[other] obiekt · { $count } klucza
    }
variables-varies = Inna wartość w każdym przebiegu: pokazana jest pierwsza
variables-not-run = Nieobliczona na tej stronie (warunek niespełniony)
variables-block = { $name }: przechwycona zawartość HTML (blok)
variables-edit = Edytuj wyrażenie
variables-copy-name = Kopiuj nazwę
variables-expression = Wyrażenie
planned-step = planowane na etap { $step }
planned-edit = Edycja: etap { $step }
pages-search = Szukaj stron
pages-count = { $count ->
        [one] { $count } strona
        [few] { $count } strony
        [many] { $count } stron
       *[other] { $count } strony
    }
pages-new = Nowa strona
pages-collection-none = Wpisy bez strony: każdy wpis może mieć własną stronę, ze wspólnego szablonu
pages-collection-create = Utwórz strony wpisów
pages-settings = Ustawienia strony
pages-name = Nazwa strony
pages-path = Adres
pages-home-path = Adres strony głównej nie zmienia się
pages-redirect = Przekieruj stary adres
pages-redirect-help = Przekierowanie trwałe (301) ze starego adresu na nowy: istniejące linki i pozycja w wyszukiwarce zostają zachowane
pages-draft = Wersja robocza
pages-draft-help = Strona robocza pozostaje edytowalna, ale nie jest publikowana
pages-home-draft = Strona główna jest zawsze publikowana
pages-draft-tag = robocza
pages-publish = Opublikuj stronę
pages-to-draft = Przenieś do roboczych
pages-seo = Wyszukiwarki
template-settings = Ustawienia szablonu
template-settings-help = Dla każdej strony kolekcji; strona może je zmienić we własnych ustawieniach.
pages-og-type = Typ po udostępnieniu
pages-og-type-help = Co strona zapowiada sieciom społecznościowym (og:type): stronę witryny lub artykuł z datą.
pages-og-type-auto = Automatyczny
pages-og-type-website = Strona
pages-og-type-article = Artykuł
pages-date = Data publikacji
pages-look = Wygląd
pages-body-class = Klasa body
pages-body-class-help = Klasy dodawane do tagu <body> strony (motyw, specjalny układ)
pages-body-class-add = Dodaj klasę
pages-theme = Motyw
pages-theme-help = Jasny lub ciemny motyw tej strony (atrybut data-theme)
pages-theme-site = Domyślny witryny
pages-theme-light = Jasny
pages-theme-dark = Ciemny
pages-file = Zawartość pliku
pages-file-help = Plik serwera, zapisywany bez zmian (.htaccess, robots.txt, PHP). Tagi szablonu są odczytywane: wartość {"{{"} site.url {"}}"}, warunek {"{%"} if env.staging {"%}"}…{"{%"} endif {"%}"}, lista {"{%"} for e in collections.jobs {"%}"}…{"{%"} endfor {"%}"}.
pages-file-computed = Zawiera elementy, obrazy lub komponenty: ustawia się to w Warstwach, nie tutaj.
pages-file-invalid = Tekst nie został zapisany: { $error }
pages-file-tags = Tagi szablonu odczytywane przy publikacji: wartości między {"{{"} {"}}"}, warunki i listy między {"{%"} {"%}"} (środowisko publikacji: env.staging, wpisy: collections.name).
pages-file-tags-none = Zapisywany bez zmian. Można w nim zapisać wartość witryny ({"{{"} site.url {"}}"}), warunek dotyczący środowiska ({"{%"} if env.staging {"%}"}…{"{%"} endif {"%}"}) lub listę wpisów ({"{%"} for e in collections.name {"%}"}…{"{%"} endfor {"%}"}).
pages-seo-title = Tytuł w wynikach wyszukiwania
pages-seo-description = Opis
pages-seo-image = Obraz do udostępniania (sieci społecznościowe)
pages-index = Widoczna dla wyszukiwarek
pages-index-help = Wyłączone: tag „noindex” i usunięcie strony z mapy witryny
pages-seo-data = Zapisane w pliku danych { $file } (można je też edytować w Pages CMS)
pages-seo-computed = Obliczane przez szablon: zmień to w logice witryny
pages-serp = Podgląd wyniku wyszukiwania
pages-duplicate = Duplikuj
pages-copy-word = kopia
pages-delete = Usuń
pages-delete-title = Usunąć „{ $name }”?
pages-delete-body = Strona i jej zawartość zostaną usunięte z witryny. Ctrl+Z ją przywraca.
pages-home-delete = Strony głównej nie można usunąć
pages-move-root = Przenieś do katalogu głównego
pages-new-here = Nowa strona tutaj
pages-folder-not-empty = Usuń (folder musi być pusty)
action-back = Wstecz
pages-toggle = Rozwiń lub zwiń
design-template = Szablon · { $collection }
design-template-help = Szablon kolekcji (układ „{ $layout }”): zmiany dotyczą wszystkich jej wpisów, poza tymi z własnym projektem
design-no-template = Wpisy bez strony
design-own = Własny projekt
design-own-help = Ten wpis ma własny projekt: zmiany szablonu już go nie dotyczą
design-detach = Utwórz własny projekt
design-detach-help = Utwórz projekt tylko dla tego wpisu (kopię szablonu, który pozostaje bez zmian)
design-attach = Wróć do szablonu
design-orphan-help = Własny projekt, którego wpis już nie istnieje
design-orphan-delete = Usuń ten projekt
design-field-label = Własny projekt (Revel)
cms-none = Brak kolekcji w tej witrynie.
cms-entry-new = Nowa treść
cms-collection-new = Nowa kolekcja
component-make = Utwórz komponent
component-make-name = Nazwa komponentu
component-make-confirm = Utwórz
component-make-none = Najpierw zaznacz element, który ma zostać komponentem.
component-missing = Nie znaleziono komponentu: { $name }
component-edit = Edytuj komponent
slot-title = Treść
slot-help = Treść umieszczona w slocie komponentu: własna komponentu lub specyficzna dla tej instancji
slot-default = Treść komponentu
slot-own = Treść specyficzna dla tej instancji
slot-edit = Edytuj tutaj
slot-edit-help = Skopiuj treść komponentu do tej instancji, aby zmienić ją tylko tutaj (teksty, dodane lub usunięte elementy)
slot-reset = Wróć do treści komponentu
slot-reset-help = Usuń własną treść instancji: znów będzie pokazywać treść komponentu
inspector-slot = Slot treści
inspector-slot-help = Każda instancja komponentu może zastąpić treść tego elementu własną (narzędzia karty, punkty listy); bez własnej treści wyświetlana jest ta
component-detach = Odłącz
component-detach-help = Zastępuje tę instancję kopią jej zawartości, którą można edytować osobno
component-props = Właściwości
component-props-empty = Powiąż tekst, obraz, link lub widoczność elementu, aby ustawiać każdą instancję osobno.
component-prop-inverted = { $label } (odwrócone)
layout-props = Właściwości układu
layout-props-empty = Powiąż tekst, obraz, link lub widoczność elementu układu, aby ustawiać go strona po stronie.
layout-props-help = Wartości wybrane dla tej strony; elementy układu (nagłówek, stopka…) łączą się z nimi z zakładki Style. Dodana właściwość dotyczy każdej strony używającej układu.
component-prop-add = Dodaj właściwość
component-prop-remove = Usuń właściwość (jej elementy zachowają wartość domyślną)
component-prop-default = Wartość domyślna
component-prop-label = Nazwa właściwości
component-prop-reset = Przywróć wartość domyślną
component-prop-computed = { $label } (obliczone)
prop-kind-text = Tekst
prop-kind-image = Obraz
prop-kind-link = Link
prop-kind-switch = Widoczność
prop-kind-list = Lista
component-bindings = Powiązane z właściwościami
component-bind = Powiąż z właściwością
component-bind-new = Nowa właściwość „{ $label }”
component-bind-field = Powiąż z polem wpisu
component-unbind = Rozłącz (zapisywana jest wartość domyślna)
component-field-url = Adres strony wpisu
component-field-fallback = { $field }, a w razie braku { $other }
component-bind-text = Tekst
component-bind-image = Obraz
component-bind-link = Link
component-bind-visibility = Widoczność
component-done = Gotowe
component-rename = Zmień nazwę
component-delete = Usuń komponent
component-delete-title = Usunąć komponent „{ $name }”?
component-empty = Ten komponent nie ma elementu do edycji.
components-none = Brak komponentów: zaznacz element, a potem „+”, aby utworzyć komponent.
components-count = { $count ->
        [0] brak komponentów
        [one] 1 komponent
        [few] { $count } komponenty
        [many] { $count } komponentów
       *[other] { $count } komponentu
    }
components-search = Szukaj komponentów
components-uses = { $count ->
        [one] Użyty raz
        [few] Użyty { $count } razy
        [many] Użyty { $count } razy
       *[other] Użyty { $count } razy
    }
components-new = Utwórz komponent z zaznaczenia
assets-search = Szukaj zasobów
assets-import = Importuj zasoby
library-new-folder = Nowy folder
library-new-subfolder = Nowy podfolder
library-folder-name = Nazwa folderu
library-open = Otwórz
library-rename = Zmień nazwę
library-delete-folder = Usuń folder
library-move-to = Przenieś do „{ $folder }”
library-move-out = Usuń z folderu
media-type-filter = Typ multimediów
media-all-types = Wszystkie typy
media-kind-image = Obrazy
media-kind-video = Filmy
media-kind-audio = Dźwięk
media-kind-document = Dokumenty
media-kind-font = Czcionki
media-kind-animation = Animacje
media-kind-other = Inne
media-sort = Sortuj
media-sort-recent = Najnowsze
media-sort-oldest = Najstarsze
media-sort-name = Nazwa (A → Z)
media-sort-name-desc = Nazwa (Z → A)
media-sort-type = Typ
media-sort-size = Rozmiar
media-thumb-size = Rozmiar miniatur
media-folder-expand = Rozwiń w miejscu
media-details = Szczegóły
media-insert = Umieść na stronie
media-insert-short = Umieść
media-url = Adres
media-size = Rozmiar
media-alt = Domyślny tekst alternatywny
media-alt-help = Co przedstawia obraz, dla czytników ekranu
media-replace = Zastąp
media-replace-help = Zastąpienie zachowuje adres: każda strona używająca tego pliku zostanie zaktualizowana. Stary plik jest przechowywany w .revel/corbeille.
media-copy-url = Kopiuj adres
media-none = Brak multimediów w folderach witryny
media-no-match = Nie znaleziono multimediów
media-online = { $count ->
    [one] 1 obraz online
    [few] { $count } obrazy online
    [many] { $count } obrazów online
   *[other] { $count } obrazu online
}
media-online-help = Obrazy linkowane z innej witryny (darmowe zdjęcia szablonu). Po skopiowaniu do witryny przestają zależeć od tamtej witryny i są optymalizowane jak Twoje multimedia.
media-online-copy = Pobierz obrazy do multimediów witryny
media-online-copying = Kopiowanie…
media-count = { $count ->
        [0] brak multimediów
        [one] 1 plik multimedialny
        [few] { $count } pliki multimedialne
        [many] { $count } plików multimedialnych
       *[other] { $count } pliku multimedialnego
    }
picker-image-title = Wybierz obraz
picker-media-title = Wybierz plik
picker-folder = Folder
picker-all-folders = Wszystkie foldery
picker-choose-image = Wybierz obraz z multimediów
picker-choose-media = Wybierz plik z multimediów
picker-add-many = { $count ->
    [0] Dodaj
    [one] Dodaj 1 element
    [few] Dodaj { $count } elementy
    [many] Dodaj { $count } elementów
   *[other] Dodaj { $count } elementu
}
picker-count-images = { $count ->
        [0] brak obrazów
        [one] 1 obraz
        [few] { $count } obrazy
        [many] { $count } obrazów
       *[other] { $count } obrazu
    }
size-kb = { $n } KB
size-mb = { $n } MB
size-gb = { $n } GB
dock-menu = Opcje panelu
dock-float = Odłącz
dock-close = Zamknij „{ $panel }”
dock-move = Przesuń okno
dock-panels = Panele
dock-show-all = Pokaż panele
dock-hide-all = Ukryj panele
dock-reset = Układ domyślny
dock-fold = Zwiń kolumnę
dock-unfold = Rozwiń kolumnę
layers-search = Szukaj warstw
layers-collapse = Zwiń wszystko
layers-parent = Zaznacz element nadrzędny
editor-zoom-help = Powiększenie (Alt+kółko; Shift+1 dopasuj, Shift+0 100%, Shift+2 zaznaczenie)
editor-zoom-fit-pct = Dopasuj ({ $pct })
editor-zoom-fit-short = Dopasuj powiększenie
editor-zoom-100 = Powiększenie 100%
editor-path = Ścieżka elementu
editor-scrub = Zmień liczbę
key-zoom-fit = Shift+1
key-zoom-100 = Shift+0
key-zoom-wheel = Alt+kółko
key-panels = Ctrl+\
key-alt-click = Alt+kliknięcie
key-drag-label = Przeciągnij etykietę
home-search = Szukaj witryn
home-view = Widok
home-view-grid = Siatka
home-view-list = Lista
action-edit-site = Otwórz edytor

# Site views (top bar)
views = Widoki witryny
view-editor = Edytor
view-cms = CMS
view-locales = Lokalizacja
view-settings = Ustawienia
nav-app-settings = Ustawienia Revel
preview-help = Pokazuje witrynę tak, jak zostanie opublikowana, na bieżącej stronie (Shift+kliknięcie: w nowym oknie)
site-settings = Ustawienia witryny
action-open-help = Otwiera folder zawierający witrynę Revel
action-import-help = Zaimportuj istniejącą witrynę: folder ze stronami HTML (także eksport z Webflow), działającą witrynę (Framer…), repozytorium Git, plik Figma, eksport WordPress lub projekt Eleventy
home-sort = Sortuj
home-sort-recent = Ostatnie
home-sort-name = Nazwa
new-site-placeholder = Moja witryna
new-site-lang = Język witryny
new-site-template = Szablon
new-site-template-blank = Pusta
new-site-template-blank-help = Jedna strona główna do wypełnienia
new-site-more = Więcej
new-site-more-help = Wszystkie szablony: według branży, katalog online, plik
new-site-template-showcase = Wizytówka
new-site-template-showcase-help = Strony główna, o nas i kontakt złożone z bloków, z menu
new-site-template-blog = Blog
new-site-template-blog-help = Strona główna, lista wpisów i wpisy edytowalne w CMS
new-site-location = Lokalizacja
new-site-change = Zmień…
new-site-create = Utwórz witrynę
new-site-creating = Tworzenie…
new-site-auto = Zgodnie z urządzeniem
new-site-scheme-help = Kolory jasne, ciemne lub oba, zależnie od ustawienia urządzenia odwiedzającego (do zmiany później w Ustawienia › Ogólne › Kolory)
template-pages = Strony: { $n }
template-source = Źródło szablonu
template-source-builtin = Wbudowane
template-source-catalog = Katalog
template-open-file = Otwórz plik
template-open-file-help = Otwórz plik szablonu (.revel-template), który otrzymałeś lub zapisałeś
template-file-kind = Szablon Revel
template-from-file = Plik szablonu
template-catalog-loading = Odczytywanie katalogu…
template-catalog-unreachable = Nie znaleziono katalogu { $repo }
template-catalog-empty = Ten katalog nie zawiera jeszcze żadnego szablonu
template-placeholders = Obrazy zastępcze do wymiany: { $n }
template-placeholders-help = Obrazy szablonu zastąpione obrazami zastępczymi online: zamień je w edytorze na własne
template-save-lead = Zapisz witrynę jako szablon: lekki plik do udostępnienia lub katalog GitHub ze wszystkimi plikami
template-save-title = Szablon
template-name = Nazwa szablonu
template-category = Kategoria
template-description = Opis
template-author = Autor
template-license = Licencja
template-license-help = Warunki ponownego użycia szablonu (teksty, układ, obrazy wysłane do katalogu)
template-license-none = Wszelkie prawa zastrzeżone
template-preview = Miniatura
template-preview-help = Obraz pokazywany w galerii szablonów (maks. 2 MB)
template-preview-none = Kolory witryny
template-target-file = Plik szablonu
template-target-file-what = Lekki: ikony i czcionki dołączone, linki do obrazów zastępczych
template-target-file-help = Plik do wysłania lub zachowania; obrazy witryny są zastępowane obrazami zastępczymi online o tym samym rozmiarze, filmy i dokumenty są pomijane
template-save-file = Zapisz…
template-saving = Zapisywanie…
template-target-catalog = Katalog GitHub
template-target-catalog-help = Wysyła kompletny szablon (z obrazami, filmami i plikami): dodawany, jeśli możesz zapisywać w repozytorium, w przeciwnym razie proponowany jako pull request
template-catalog-help = Repozytorium GitHub z katalogiem (Ustawienia › Szablony)
template-needs-github = Połącz konto GitHub (Ustawienia › Profil i konta)
template-send = Wyślij
template-sending = Wysyłanie…
template-saved = Szablon zapisany · { $size } · obrazy zastępcze: { $images } · pominięte pliki multimedialne: { $missing }
template-published = Szablon dodany do katalogu · pliki: { $files } · { $size }
template-published-pr = Pull request otwarty · pliki: { $files } · { $size }
settings-templates = Szablony
settings-templates-help = Wspólny katalog szablonów na GitHub, oferowany w „Nowa witryna”
template-catalog = Katalog
template-catalog-count = Szablony: { $n }
template-catalogs = Katalogi
template-catalogs-help = Repozytoria GitHub, których szablony pojawiają się w Nowa witryna › Więcej › Katalog
template-catalogs-none = Brak katalogu
template-catalog-remove = Usuń ten katalog
template-catalog-add = Dodaj
template-catalog-add-help = Dodaj katalog szablonów GitHub
template-catalog-add-placeholder = właściciel/repozytorium
template-catalog-invalid = Wpisz właściciel/repozytorium lub adres github.com
template-catalog-missing = Nie znaleziono
template-catalog-missing-help = Repozytorium nie istnieje, jest prywatne lub nie ma pliku catalog.json
template-catalog-open = Otwórz w GitHub
template-catalog-own = Twój katalog
template-catalog-create = Utwórz katalog na GitHub
template-catalog-create-help = Tworzy publiczne repozytorium „revel-templates” na Twoim koncie GitHub, z plikiem catalog.json, i wybiera je jako katalog
template-catalog-creating = Tworzenie…
error-template-file = Nieczytelny plik szablonu ({ $detail })
error-template-version = Szablon zapisany przez nowszą wersję Revel ({ $detail }): zaktualizuj Revel
error-template-catalog = Katalog szablonów jest niedostępny ({ $detail })
error-template-catalog-missing = Szablonu „{ $detail }” nie ma w katalogu
error-template-publish = Wysyłanie do katalogu zostało przerwane ({ $detail })
template-cat-all = Wszystkie
template-cat-business = Biznes
template-cat-creative = Kreatywne
template-cat-commerce = Sklep
template-cat-editorial = Redakcyjne
template-cat-food = Jedzenie i napoje
template-cat-events = Wydarzenia
template-cat-travel = Noclegi
template-cat-services = Usługi
template-cat-education = Edukacja
template-cat-community = Społeczność
template-cat-personal = Osobiste

# Site settings
settings-general = Ogólne
settings-publish = Publikacja
settings-export = Eksport
settings-seo = SEO
settings-redirects = Przekierowania
settings-fonts = Czcionki
settings-code = Własny kod
settings-forms = Formularze
settings-versions = Wersje
versions-help = Zapisane wersje witryny (repozytorium Git jej folderu): porównuj, przywracaj, synchronizuj z repozytorium GitHub witryny (commity Pages CMS)
versions-changes = { $n ->
    [one] 1 zmiana
    [few] { $n } zmiany
    [many] { $n } zmian
   *[other] { $n } zmiany
}
versions-changes-help = Pliki zmienione od ostatniej wersji
versions-clean = Aktualne
versions-sync = Synchronizuj
versions-sync-help = Zapisz zmiany, pobierz wersje online (Pages CMS), a potem wyślij wersje witryny
versions-sync-account = Połącz konto GitHub w ustawieniach aplikacji
versions-ahead = Wersje do wysłania
versions-behind = Wersje online do pobrania
versions-save = Zapisz wersję
versions-message = Opis
versions-default-message = Wersja z { $date }
versions-first = Pierwsza wersja
versions-merge-message = Wersje pobrane z repozytorium online
versions-init = Włącz wersje
versions-init-help = Folder witryny staje się repozytorium Git: każda wersja zapisuje wszystkie jego pliki
versions-remote = Repozytorium GitHub
versions-remote-help = Adres HTTPS repozytorium witryny, synchronizowanego z Pages CMS
versions-branch = Gałąź
versions-conflict = Te same wiersze zmieniono online i w witrynie: nic nie zostało zmienione.
versions-keep-ours = Zachowaj wersję witryny
versions-keep-theirs = Przyjmij wersję online
versions-resolved-ours = Konflikt rozwiązany: zachowano wersję witryny
versions-resolved-theirs = Konflikt rozwiązany: zachowano wersję online
versions-current = Bieżące zmiany
versions-current-help = Jeszcze niezapisane
versions-latest = Najnowsza
versions-files = Zmienione pliki
versions-restore = Przywróć
versions-restore-help = Przywróć pliki tej wersji (bieżący stan jest najpierw zapisywany)
versions-restore-title = Przywrócić „{ $name }”?
versions-before-restore = Przed przywróceniem
versions-restored = Przywrócenie „{ $name }”
versions-no-files = Żaden plik nie został zmieniony
versions-added = Dodano
versions-deleted = Usunięto
versions-modified = Zmodyfikowano
versions-binary = Plik binarny
versions-more-lines = Więcej wierszy: { $n }

# Team work (D-89)
versions-team = Praca zespołowa
versions-team-help = Każdy pracuje na własnej kopii witryny, a potem synchronizuje: wersje innych są pobierane, Twoje wysyłane. Zmiany wprowadzone w różnych miejscach są łączone; miejsce zmienione po obu stronach wybierasz Ty.
versions-team-title = Pracuj zespołowo
versions-mode-solo = Samodzielnie
versions-mode-solo-help = Wersje przechowywane w folderze witryny, na tym komputerze
versions-mode-shared = Folder współdzielony
versions-mode-shared-help = Wspólne wersje w folderze sieci lokalnej (NAS, serwer, udział Windows): bez konta, bez dostawcy
versions-mode-online = Repozytorium online
versions-mode-online-help = Wspólne wersje w GitHub, GitLab lub innej usłudze Git: dostępne z każdego miejsca, treść edytowalna w przeglądarce za pomocą Pages CMS
versions-compare-where = Wspólne wersje
versions-compare-where-help = Miejsce, w którym wszyscy wysyłają i pobierają wersje; każdy komputer przechowuje też pełną kopię witryny i może pracować offline
versions-compare-where-shared = Folder sieciowy (NAS, serwer)
versions-compare-where-online = GitHub, GitLab…
versions-compare-accounts = Konta
versions-compare-accounts-help = Co jest potrzebne do wysyłania i pobierania wersji. W GitHub każdy ma własne darmowe konto, zaproszone do repozytorium (Settings › Collaborators, bez limitu): regulamin GitHub zabrania współdzielenia loginu.
versions-compare-accounts-shared = Brak, dostęp do folderu
versions-compare-accounts-online = Jedno konto GitHub na osobę, zaproszone do repozytorium
versions-compare-remote = Poza biurem
versions-compare-remote-help = Synchronizacja spoza sieci lokalnej
versions-compare-remote-shared = Nie, chyba że przez VPN
versions-compare-remote-online = Tak
versions-compare-browser = Treść edytowana w przeglądarce
versions-compare-browser-help = Pages CMS edytuje treści (wpisy, artykuły…) bezpośrednio w repozytorium online, bez Revel
versions-compare-browser-shared = Nie
versions-compare-browser-online = Tak, z Pages CMS
versions-compare-backup = Kopie witryny
versions-compare-backup-help = Miejsca przechowujące każdą wersję witryny
versions-compare-backup-shared = NAS i każdy komputer
versions-compare-backup-online = Online i każdy komputer
versions-compare-provider = Dostawca
versions-compare-provider-help = Usługa zewnętrzna, od której zależy praca zespołowa
versions-compare-provider-shared = Brak
versions-compare-provider-online = GitHub, GitLab…
versions-shared-folder = Folder współdzielony
versions-shared-folder-help = Folder sieciowy przechowujący wspólne wersje: Revel tworzy tam „nazwa-witryny.git”, a następnie wysyła wersje. Pozostali otwierają witrynę przez Ekran główny › Importuj › Git, podając ten folder.
versions-shared-choose = Wybierz…
versions-online-url = Adres repozytorium
versions-online-url-help = Adres HTTPS repozytorium (GitHub, GitLab…); prywatne repozytorium GitHub: konto GitHub połączone w Ustawieniach
versions-team-on = Włącz
versions-team-off-title = Zatrzymać pracę zespołową?
versions-team-off = Zatrzymaj
versions-team-copy = Kopiuj adres dla zespołu (Importuj › Git)
versions-team-copied = Adres skopiowany
versions-items = { $n ->
    [one] Jedno miejsce zmienione po obu stronach
    [few] { $n } miejsca zmienione po obu stronach
    [many] { $n } miejsc zmienionych po obu stronach
   *[other] { $n } miejsca zmienione po obu stronach
}
versions-items-help = Pozostałe zmiany zostały połączone. Wybierz wersję do zachowania dla każdego miejsca, a następnie zastosuj: wynik jest zapisywany i wysyłany.
versions-pick-ours = Moja
versions-pick-theirs = Otrzymana
versions-pick-all-ours = Wszystkie moje
versions-pick-all-theirs = Wszystkie otrzymane
versions-apply = Zastosuj
versions-removed = Usunięto
versions-other-files = Inne pliki
versions-resolved-items = Rozwiązano miejsca zmienione po obu stronach
versions-area-page = Strona { $name }
versions-area-layout = Układ „{ $name }”
versions-area-component = Komponent „{ $name }”
versions-area-collection = Kolekcja „{ $name }”
versions-area-styles = Style witryny
versions-area-settings = Ustawienia witryny
versions-area-site = Witryna
versions-area-data = Plik { $name }
time-now = Przed chwilą
time-min = { $n } min temu
time-hours = { $n ->
    [one] 1 godz. temu
   *[other] { $n } godz. temu
}
settings-seo-planned = Tytuł, opis i obraz do udostępniania każdej strony, mapa witryny i robots.txt.
settings-forms-planned = Odbieranie zgłoszeń z formularzy witryny (e-mail, arkusz kalkulacyjny, usługa zewnętrzna).
settings-identity = Tożsamość
settings-site-name = Nazwa
settings-site-url = Adres publiczny
settings-site-lang = Język
settings-site-url-help = Adres opublikowanej witryny (https://…): linki między językami, mapa witryny, udostępnianie
settings-site-lang-help = Język stron witryny (atrybut lang); pozostałe języki zarządzane są w Lokalizacji
settings-typography = Typografia francuska
settings-typography-help = Spacje niełamliwe przed : ; ! ? i wewnątrz « », apostrof typograficzny, w tekstach francuskich
code-help = Dodawany bez zmian do każdej opublikowanej strony i do podglądu; nie jest uruchamiany w edytorze
code-head = Head
code-head-help = Przed </head>: meta tagi, arkusze stylów, analityka
code-body = Koniec strony
code-body-help = Przed </body>: skrypty
code-pages = Strony z własnym kodem
code-pages-help = Kod dodawany po kodzie witryny, w ustawieniach tych stron
page-code = Własny kod
page-code-help = Dodawany do tej strony po kodzie witryny (Ustawienia › Własny kod)
fonts-help = Pliki kopiowane do witryny i przez nią serwowane: żadnych zapytań odwiedzających do Google
fonts-import = Importuj pliki
fonts-google = Google Fonts
fonts-google-help = Dodaj czcionkę Google (pliki pobierane jednorazowo do witryny)
fonts-none = Brak czcionek zadeklarowanych w witrynie
fonts-italic = kursywa
fonts-unused = Nieużywana
fonts-uses = { $n ->
    [one] 1 reguła
    [few] { $n } reguły
    [many] { $n } reguł
   *[other] { $n } reguły
}
fonts-remove = Usuń czcionkę
fonts-remove-title = Usunąć „{ $name }” z witryny?
fonts-search = Szukaj czcionek
fonts-loading = Wczytywanie katalogu…
fonts-variants = Grubości
fonts-add-help = Pliki WOFF2 wybranych grubości pobierane do witryny, dla języków witryny
fonts-family = Rodzina
fonts-variable = Czcionka zmienna: każda grubość w zakresie
settings-site-folder = Folder
redirects-add = Dodaj przekierowanie
redirects-none = Brak przekierowań.
redirects-from = Stary adres
redirects-to = Nowy adres
redirects-help = Stary adres jest trwale przekierowywany (301) na nowy. Zmiana adresu opublikowanej strony dodaje takie przekierowanie.
redirects-remove = Usuń przekierowanie
seo-share = Udostępnianie i przeglądarka
seo-share-help = Tytuł, opis i obraz stron, które ich nie mają, ikony przeglądarki i kolor; zapisywane w każdej stronie przez Revel.
seo-site-title = Wyświetlana nazwa witryny
seo-site-title-help = Dodawana do tytułu każdej strony („Strona | Nazwa”) i podawana przy udostępnianiu.
seo-description = Opis domyślny
seo-description-help = Dla stron bez własnego opisu.
seo-share-image = Domyślny obraz do udostępniania
seo-share-image-help = Dla stron bez obrazu do udostępniania i bez obrazu okładki (zalecane 1200 × 630 px).
seo-theme-color = Kolor przeglądarki
seo-theme-color-help = Kolor paska adresu na telefonach.
seo-favicon = Ikona witryny
seo-favicon-help = Plik SVG lub PNG pokazywany na karcie przeglądarki.
seo-favicon-png = Ikona PNG
seo-favicon-png-help = Ikona zapasowa dla przeglądarek bez obsługi SVG (192 × 192 px).
seo-touch-icon = Ikona ekranu głównego
seo-touch-icon-help = Ikona skrótu na iPhonie i iPadzie (180 × 180 px).
seo-org = Organizacja
seo-org-help = Karta organizacji odczytywana przez wyszukiwarki (dane strukturalne schema.org); artykuły wskazują ją jako swojego wydawcę.
seo-org-name = Nazwa
seo-org-alt = Inna nazwa
seo-org-logo = Logo
seo-org-email = E-mail
seo-org-phone = Telefon
seo-org-founded = Rok założenia
seo-org-street = Adres
seo-org-postcode = Kod pocztowy
seo-org-city = Miasto
seo-org-country = Kraj (kod)
seo-org-social = Sieci społecznościowe
seo-org-social-help = Jeden adres w wierszu.
seo-sitemap = Mapa witryny (sitemap.xml)
seo-sitemap-lastmod = Daty aktualizacji
seo-sitemap-lastmod-help = Każdy adres w mapie witryny zawiera datę publikacji strony lub wpisu (pole „date”), jeśli ją ma.
seo-sitemap-help = Tworzona przy publikacji: wszystkie opublikowane strony, z wyjątkiem ukrytych przed wyszukiwarkami.
seo-sitemap-own = Witryna ma własną stronę sitemap.xml: to ona jest publikowana.
seo-sitemap-no-url = Brak adresu witryny (sekcja Ogólne): nie można utworzyć mapy witryny.
seo-robots-help = Puste: zezwól na wszystko i wskaż mapę witryny.
seo-robots-own = Witryna ma własną stronę robots.txt: to ona jest publikowana.
seo-pages-help = Tytuł, opis, obraz do udostępniania i widoczność każdej strony: ustawienia strony (panel Strony).
lang-fr = Francuski
lang-en = Angielski
lang-de = Niemiecki
lang-es = Hiszpański
lang-it = Włoski
lang-pt = Portugalski
lang-nl = Niderlandzki

# Localization
locales-add = Dodaj język
locales-add-help = Nowy język: adresy /kod/…, teksty tłumaczone tutaj, w edytorze i w CMS
locales-other = Inny język…
locales-code-input = Kod języka (en, de, pt-BR…)
locales-lang = Język
locales-code = Kod
locales-prefix = Adres
locales-primary = Język główny
locales-progress = Tłumaczenie
locales-progress-help = Przetłumaczone teksty: { $done } z { $total }
locales-published = Opublikowany
locales-published-help = Publikowany razem z witryną; w przeciwnym razie w przygotowaniu (widoczny tylko w edytorze i podglądzie)
locales-rename = Zmień nazwę
locales-name = Wyświetlana nazwa (przełącznik języka)
locales-remove = Usuń język
locales-remove-title = Usunąć „{ $name }”?
locales-remove-body = Teksty stron przetłumaczone na ten język zostaną usunięte (Ctrl+Z, aby cofnąć). Jego pliki danych i wpisy treści pozostają w folderze witryny.
locales-no-url = Brak adresu witryny (Ustawienia › Ogólne): linki między językami (hreflang) nie są dodawane do stron.
locales-part-pages = Strony
locales-part-pages-help = Teksty zapisane na stronach, w układach i komponentach
locales-part-data = Dane
locales-part-data-help = Teksty plików danych, zapisane w folderze języka
locales-part-entries = Treść
locales-part-entries-help = Wpisy kolekcji, tłumaczone w CMS
locales-search = Szukaj tekstów
locales-todo = Do przetłumaczenia
locales-todo-help = Tylko teksty do przetłumaczenia
locales-keep-source = Zachowaj oryginalny tekst (tłumaczenie niepotrzebne)
locales-translation = Tłumaczenie
locales-none = Brak tekstów
locales-place-layout = Układ { $name }
locales-entry-done = Przetłumaczone
locales-auto = Tłumacz automatycznie
locales-auto-help = Tłumaczy dotąd nieprzetłumaczone teksty, na tym komputerze, offline
locales-auto-none = Brak modelu dla { $from } → { $to }: pobierz język w Ustawienia › Tłumaczenie automatyczne
locales-auto-nothing = Wszystko jest już przetłumaczone
locales-auto-title = Tłumacz automatycznie
locales-auto-texts = { $n ->
    [one] 1 tekst
    [few] { $n } teksty
    [many] { $n } tekstów
   *[other] { $n } tekstu
}
locales-auto-entries = { $n ->
    [one] 1 wpis
    [few] { $n } wpisy
    [many] { $n } wpisów
   *[other] { $n } wpisu
}
locales-auto-keep = Słowa do zachowania bez zmian
locales-auto-keep-help = Nazwy marek lub produktów, oddzielone przecinkami. Nazwa witryny jest zawsze zachowywana.
locales-auto-start = Tłumacz
locales-auto-running = Tłumaczenie { $from } → { $to }
locales-auto-eta-min = około { $n } min
locales-auto-eta-less = mniej niż minuta
locales-auto-eta-wait = szacowanie…
locales-auto-stop = Zatrzymaj
locales-auto-stop-help = Zatrzymaj po bieżącej partii (to, co przetłumaczone, zostaje zachowane)
locales-auto-stopping = Zatrzymywanie…
locales-review = Do sprawdzenia
locales-review-help = Tłumaczenie maszynowe do sprawdzenia
locales-review-ok = Oznacz jako sprawdzone
locales-review-only = Do sprawdzenia
locales-review-only-help = Tylko tłumaczenia maszynowe do sprawdzenia
locales-review-all = Oznacz wszystko jako sprawdzone
locales-progress-auto-help = Przetłumaczone teksty: { $done } z { $total }, do sprawdzenia: { $auto }
action-add = Dodaj
action-rename = Zmień nazwę
action-move-up = Przesuń w górę
action-move-down = Przesuń w dół
colors-opacity = Krycie
colors-pick = Wybierz kolor
seo-org-logo-help = Logo organizacji pokazywane przez wyszukiwarki (najlepiej kwadratowe)
seo-org-social-add = Dodaj link (LinkedIn, Instagram…), a potem Enter
action-remove = Usuń
rows-item = Wartość { $n }
rows-add = Dodaj wartość (Enter)
editor-locale = Wyświetlany język
editor-locale-help = Język wyświetlany na kanwie
editor-locale-other-help = Edytowane teksty są zapisywane w tym języku; struktura i style są wspólne dla wszystkich języków
cms-locale-help = Język wyświetlanych wpisów
cms-locale-shared = Pola i wpisy są wspólne dla wszystkich języków: zmieniaj je w języku głównym
cms-untranslated = Do przetłumaczenia
cms-untranslated-help = Jeszcze nieprzetłumaczone: treść w języku głównym; edycja tworzy tłumaczenie
cms-auto = Tłumaczenie maszynowe

# CMS view
cms-collections = Kolekcje
cms-search = Szukaj treści
cms-fields-edit = Pola
cms-file = Plik
cms-no-entries = Brak treści.
cms-draft = Wersja robocza
cms-open-page = Otwórz stronę w edytorze
cms-shown = { $shown } z { $total }
cms-entry = Treść
cms-body = Tekst
cms-items = { $count ->
    [one] { $count } wpis
    [few] { $count } wpisy
    [many] { $count } wpisów
   *[other] { $count } wpisu
}
cms-fields = { $count ->
    [one] { $count } pole
    [few] { $count } pola
    [many] { $count } pól
   *[other] { $count } pola
}

# Add panel (groups, tooltip help, planned items)
insert-group-structure = Struktura
insert-group-text = Tekst
insert-group-media = Multimedia
insert-group-cms = CMS
insert-group-forms = Formularze
insert-group-code = Kod
insert-help-section = Duża część strony (baner, wstęp…). Przeciągnij na stronę, aby ją umieścić, lub kliknij dwukrotnie, aby dodać wewnątrz zaznaczenia.
insert-help-container = Pudełko grupujące i wyrównujące inne elementy. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-heading = Nagłówek sekcji (h2). Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-paragraph = Blok tekstu. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-link = Klikalny tekst prowadzący do strony lub adresu. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-button = Link w stylu przycisku. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-image = Zoptymalizowany obraz (AVIF, WebP) w kilku rozmiarach. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-list = Lista punktowana. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-grid = Siatka dwukolumnowa: każda komórka zawiera elementy. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-columns = Kolumny obok siebie, które układają się jedna pod drugą, gdy zabraknie miejsca. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-rich-text = Nagłówek, akapit i lista do edycji w miejscu. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-video = Film witryny: wybierz plik w jego ustawieniach. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-icon = Ikona liniowa w kolorze tekstu: wybierz rysunek w jej ustawieniach. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-embed = Kod HTML zapisywany bez zmian na stronie (osadzenie usługi, mapa…). Przeciągnij na stronę lub kliknij dwukrotnie.
insert-help-collection-list = Jeden element na każdy wpis kolekcji, z filtrem, sortowaniem i licznikiem. Przeciągnij na stronę lub kliknij dwukrotnie.
insert-text-rich = Nagłówek
    Akapit tekstu.
insert-text-embed = Kod HTML
insert-text-collection = Widok
block-icon = Ikona
block-embed = Kod HTML
block-embed-help = Zapisywany bez zmian na stronie. Skrypty uruchamiają się tylko w podglądzie i w opublikowanej witrynie.
block-video-src = Źródło
block-video-src-help = Plik wideo witryny (MP4, WebM) lub adres filmu
block-video-poster = Obraz okładki
block-video-poster-help = Obraz pokazywany przed odtwarzaniem
block-video-muted = Bez dźwięku
block-video-preload = Ładowanie
block-video-preload-help = Co przeglądarka ładuje przed odtwarzaniem
block-video-preload-default = Wybór przeglądarki
block-video-preload-none = Nic (przy odtwarzaniu)
block-video-preload-metadata = Czas trwania i pierwsza klatka
block-video-preload-auto = Cały film
picker-choose-video = Wybierz film z multimediów
field-group = Pole formularza
field-group-help = Co wpisuje odwiedzający i pod jaką nazwą wysyłana jest odpowiedź
field-type = Typ
field-type-text = Tekst
field-type-email = E-mail
field-type-tel = Telefon
field-type-url = Adres internetowy
field-type-number = Liczba
field-type-date = Data
field-type-checkbox = Pole wyboru
field-type-radio = Wybór pojedynczy
field-type-file = Załącznik
field-type-password = Hasło
field-type-hidden = Ukryte
field-name = Nazwa wysyłana
field-name-help = Nazwa odpowiedzi w otrzymanej wiadomości (litery, cyfry, myślniki)
field-required = Wymagane
field-required-help = Formularz nie zostanie wysłany, dopóki to pole jest puste
field-placeholder = Tekst zastępczy
field-placeholder-help = Szary tekst pokazywany w pustym polu
field-rows = Widoczne wiersze
field-auto = Autouzupełnianie
field-auto-help = Co przeglądarka może zaproponować do uzupełnienia
field-auto-default = Wybór przeglądarki
field-auto-off = Wyłączone
field-auto-name = Imię i nazwisko
field-auto-given-name = Imię
field-auto-family-name = Nazwisko
field-auto-email = E-mail
field-auto-tel = Telefon
field-auto-organization = Firma
field-auto-street = Adres
field-auto-postal-code = Kod pocztowy
field-auto-city = Miasto
field-auto-country = Kraj
block-video = Odtwarzanie
block-video-controls = Elementy sterowania odtwarzaniem
block-video-autoplay = Autoodtwarzanie (bez dźwięku)
block-video-loop = Zapętlenie
block-video-player = Wideo
block-video-player-url = Adres
block-video-player-help = Link do filmu na YouTube lub Vimeo albo plik wideo witryny; po naciśnięciu zastępuje okładkę
block-video-player-file = Plik witryny
block-list = Lista kolekcji
block-list-computed = Lista obliczana przez wyrażenie: edytowalna w „Lista” powyżej.
block-list-collection = Kolekcja
block-list-also-add = + Kolejna kolekcja
block-list-also-remove = Usuń tę kolekcję z listy
block-list-also-help = Wpisy innych kolekcji odczytywane po niej, w kolejności kolekcji (na przykład projekty i aktualności wybierane razem).
block-list-filter = Filtr
block-list-items = Wpisy
block-list-items-all = Wszystkie
block-list-items-pick = Wybrane
block-list-first = Wyróżnione wpisy
block-list-first-help = Te wpisy najpierw, po kolei, a potem pozostałe; przy ustawionej maksymalnej liczbie pozostałe uzupełniają listę (propozycje projektu uzupełnione najnowszymi)
block-list-first-none = Brak
block-list-first-pick = Wybrane wpisy
block-list-first-field = Wpisy z pola „{ $field }”
block-list-first-full = Tylko jeśli wypełniają listę
block-list-first-full-help = Przy ustawionej maksymalnej liczbie: jeśli wybranych wpisów jest mniej niż ta liczba, wyświetlana jest zwykła lista.
block-list-contains = Tylko jeśli pole
block-list-contains-help = Wpisy, których pole listy zawiera bieżącą stronę lub wartość
block-list-contains-none = –
block-list-value-yes = Tak
block-list-value-no = Nie
block-list-contains-value-help = Wpisy, których pole zawiera tę wartość; puste: wartość wyświetlanej strony
block-list-contains-value = zawiera
block-list-contains-current = Bieżąca strona
block-list-all = Wszystkie wpisy
block-list-filter-value = Wartość pola
block-list-filter-any = (wypełnione)
block-list-sort = Sortowanie
block-list-sort-default = Kolejność kolekcji
block-list-order = Kolejność
block-list-asc = Rosnąco
block-list-desc = Malejąco
block-list-limit = Liczba wpisów
block-list-limit-all = Wszystkie
block-list-more = Liczba przed „Pokaż więcej”
block-list-more-help = Liczba wpisów pokazanych na początku; kolejne pojawiają się w tej samej liczbie po dojściu do końca listy (puste: wszystkie naraz)
block-list-date = Data
block-bound = Powiązane z wpisem
block-bound-text = Tekst
block-bound-text-label = Tekst
block-bound-image = Obraz
block-bound-link = Link
block-bound-written = Wpisany tekst
block-bound-none = Brak
block-bound-page = Strona wpisu
insert-group-blocks = Bloki
insert-block-hero = Sekcja otwierająca
insert-block-features = Funkcje
insert-block-text-image = Tekst i obraz
insert-block-cta = Wezwanie do działania
insert-block-testimonial = Opinia
insert-block-faq = Pytania
insert-block-contact = Dane kontaktowe
insert-help-block = Kompletna sekcja gotowa do edycji (teksty, regulowane style). Przeciągnij na stronę lub kliknij dwukrotnie.
insert-cart = Koszyk i zamówienie
insert-help-cart = Koszyk witryny i formularz zamówienia wysyłany e-mailem (formularz witryny, bez płatności online). Przyciski „Dodaj do koszyka” go wypełniają.
insert-cart-add = Dodaj do koszyka
insert-help-cart-add = Przycisk dodający produkt do koszyka witryny: produkt i cena wpisane lub powiązane z polami wpisu.
insert-text-cart-add = Dodaj do koszyka
insert-grid = Siatka
insert-columns = Kolumny
insert-rich-text = Tekst sformatowany
insert-video = Wideo
insert-icon = Ikona
insert-lottie = Lottie
insert-collection-list = Lista kolekcji
insert-locale-switch = Przełącznik języka
insert-help-locale-switch = Jeden link na język witryny do tej samej strony (bieżący język wyróżniony)
insert-text-locale-switch = Języki
insert-form = Formularz
insert-embed = Kod HTML

# Canvas: selection navigation, moves, view
editor-select-child = Zaznacz pierwszy element podrzędny
editor-select-parent = Zaznacz element nadrzędny
editor-select-sibling = Następny, poprzedni element
editor-move = Przesuń element
editor-move-copy = Kopiuj podczas przesuwania
editor-order = Przesuń w górę, w dół
editor-zoom-selection = Powiększ do zaznaczenia
editor-pan = Przesuń widok
key-child = Enter
key-parent = Shift+Enter
key-siblings = Tab · Shift+Tab
key-move-drag = Przeciąganie
key-copy-drag = Alt+przeciąganie
key-order = Alt+↑ · Alt+↓
key-wrap = Ctrl+G
key-zoom-sel = Shift+2
key-pan = Spacja+przeciąganie
editor-copy = Kopiuj
editor-cut = Wytnij
editor-paste = Wklej
editor-unwrap = Rozpakuj
key-copy = Ctrl+C
key-cut = Ctrl+X
key-paste = Ctrl+V
key-unwrap = Ctrl+Shift+G
cms-entry-title = Tytuł
action-create = Utwórz
cms-delete-title = Usunąć „{ $name }”?
cms-delete-body = Plik treści zostanie usunięty. Ctrl+Z go przywraca, dopóki witryna jest otwarta.
cms-gallery-add = Dodaj
cms-gallery-add-help = Wybierz jeden lub więcej obrazów z multimediów (lub upuść tu pliki)
cms-gallery-replace = Kliknij, aby zastąpić
cms-gallery-remove = Usuń z listy
cms-gallery-item = Obraz { $n }: { $name }
cms-gallery-count = { $count ->
    [0] Brak obrazów
    [one] 1 obraz
    [few] { $count } obrazy
    [many] { $count } obrazów
   *[other] { $count } obrazu
}
cms-gallery-count-files = { $count ->
    [0] Brak plików
    [one] 1 plik
    [few] { $count } pliki
    [many] { $count } plików
   *[other] { $count } pliku
}
cms-gallery-hint = przeciągnij, aby zmienić kolejność
cms-panel-resize = Przeciągnij, aby zmienić rozmiar panelu (dwukrotne kliknięcie: szerokość domyślna)
cms-template = Szablon strony
cms-template-edit = Edytuj szablon strony
cms-template-help = Układ wspólny dla każdej strony kolekcji: jego edycja aktualizuje wszystkie jej strony, poza tymi z własnym projektem
cms-template-page = Szablon stron · { $name }
cms-pin-on = Pokaż to pole jako kolumnę tabeli
cms-pin-off = Usuń to pole z kolumn tabeli
cms-columns = Kolumny tabeli
cms-column-hide = Ukryj kolumnę „{ $name }”
cms-column-left = Przesuń w lewo
cms-column-right = Przesuń w prawo
cms-column-drag = Przeciągnij, aby przenieść kolumnę
cms-column-widths-reset = Szerokości automatyczne
cms-column-resize = Przeciągnij, aby ustawić szerokość (dwukrotne kliknięcie: automatyczna)
cms-media-path = Adres (lub wybierz z multimediów)
cms-tag-duplicate = Ta wartość jest już na liście
cms-tag-help = Przeciągnij, aby zmienić kolejność (Alt+strzałki); kliknij dwukrotnie, aby edytować
cms-ref-order-help = Przeciągnij, aby zmienić kolejność wyświetlania (Alt+strzałki)
cms-ref-full = Osiągnięto maksimum ({ $max }): usuń wpis, aby dodać kolejny
cms-repeat-fold-all = Zwiń wszystko
cms-repeat-unfold-all = Rozwiń wszystko
cms-repeat-drag = Przeciągnij, aby zmienić kolejność
cms-repeat-duplicate = Duplikuj
cms-pattern-none = Tekst dowolny
cms-pattern-slug = Adres (małe litery i myślniki)
cms-pattern-email = Adres e-mail
cms-pattern-url = Link (https://…)
cms-pattern-phone = Numer telefonu
cms-pattern-postcode = Kod pocztowy (5 cyfr)
cms-pattern-custom = Własny (wyrażenie)
cms-tag-add = Dodaj…

# Rich text (step 3.2)
rt-toolbar = Formatowanie
rt-block = Typ akapitu
rt-p = Akapit
rt-h2 = Nagłówek 2
rt-h3 = Nagłówek 3
rt-h4 = Nagłówek 4
rt-quote = Cytat
rt-bold = Pogrubienie (Ctrl+B)
rt-italic = Kursywa (Ctrl+I)
rt-link = Link (Ctrl+K); pusty adres usuwa link
rt-link-apply = Zastosuj
rt-ul = Lista punktowana
rt-ol = Lista numerowana
rt-clear = Wyczyść formatowanie
rt-source-md = Wyświetl i edytuj Markdown
rt-source-html = Wyświetl i edytuj HTML
cms-body-empty = Wpisz tekst treści…
export-title = Eksportuj witrynę
export-lead = Pliki witryny (strony, style, zoptymalizowane obrazy) skopiowane do folderu, aby umieścić ją gdzie indziej lub zarchiwizować. Podgląd i publikacja tego nie wymagają.
export-env-help = Zmienne wybranego środowiska (adres witryny, ustawienia staging…)
export-images-help = Warianty AVIF i WebP w kilku rozmiarach, jak w opublikowanej witrynie; za pierwszym razem wolniej
export-where = Folder
export-run = Eksportuj
export-running = Eksportowanie…
export-reveal = Pokaż folder
export-failed = Eksport nie powiódł się. { $message }
publish-sync-versions = Synchronizuj wersje przed publikacją
publish-sync-versions-help = Zapisuje zmiany, pobiera te wykonane gdzie indziej (Pages CMS) i wysyła je do repozytorium witryny (Ustawienia › Wersje)
publish-syncing = Synchronizowanie wersji…
publish-sync-message = Publikacja
publish-building = Przygotowywanie plików witryny…

# Collection schema (step 3.3)
cms-required = Pole wymagane
cms-required-mark = Wymagane
cms-pattern = Nieprawidłowy format
cms-sub-default = Tekst
cms-sub-new = Nowe pole grupy
cms-sub-add = Dodaj pole
cms-sub-remove = Usuń pole (wartości pozostają we wpisach)
cms-sub-locked = Zdefiniowane przez komponent .pages.yml: edytowalne w tym pliku.
cms-block-new = Nowy typ bloku
cms-block-add = Dodaj typ bloku
cms-block-remove = Usuń ten typ bloku (zapisane bloki pozostają we wpisach)
cms-repeat-up = Przesuń w górę
cms-repeat-down = Przesuń w dół
cms-repeat-remove = Usuń
cms-repeat-add = Dodaj
cms-repeat-add-block = Dodaj blok
cms-json = Nieprawidłowa wartość: { $detail }
cms-more = Inne wartości ({ $count })
cms-more-help = Ukryte pola i wartości nieobecne w schemacie (.pages.yml)
cms-undeclared = Brak w .pages.yml: Pages CMS usunie to przy zapisie tej treści.
cms-undeclared-add = Dodaj do schematu
cms-undeclared-name = Nazwa odrzucona przez Pages CMS (tylko litery, cyfry, - i _)
cms-fields-help = Pola kolekcji (schemat wspólny z Pages CMS)
cms-fields-count = { $count ->
    [one] { $count } pole
    [few] { $count } pola
    [many] { $count } pól
   *[other] { $count } pola
}
cms-schema-inferred = Pola wywnioskowane z treści: pierwsza zmiana opisuje je w .pages.yml (Pages CMS).
cms-field-new = Nowe pole
cms-field-new-title = Nowe pole: { $type }
cms-field-label = Etykieta
cms-field-name = Nazwa w plikach
cms-field-name-help = Klucz zapisywany w każdym pliku treści; nie zmienia się po utworzeniu pola.
cms-field-type = Typ
cms-field-component = Zdefiniowane przez komponent „{ $name }” w .pages.yml, wspólny z innymi polami.
cms-field-help = Pomoc
cms-field-help-placeholder = Tekst pokazywany po najechaniu na etykietę
cms-field-required = Wymagane
cms-field-list = Kilka wartości
cms-field-hidden = Ukryte w formularzu
cms-field-hidden-help = Zachowane w treści bez możliwości wpisywania, tutaj i w Pages CMS.
cms-field-column = Kolumna tabeli
cms-field-column-chip = Kolumna
cms-field-primary = Tytuł treści
cms-field-primary-help = Nazwa treści w tabeli i na górze jej formularza.
cms-field-primary-chip = Tytuł
cms-field-drag = Przeciągnij, aby zmienić kolejność
cms-field-up = Przesuń w górę
cms-field-down = Przesuń w dół
cms-field-remove = Usuń
cms-field-remove-title = Usunąć pole „{ $name }”?
cms-field-remove-body = Znika z formularza i z Pages CMS; wpisane już wartości pozostają w treści. Ctrl+Z je przywraca.
cms-field-retype-title = Zmienić typ „{ $name }”?
cms-field-retype-body = Wpisane już wartości nie są konwertowane.
cms-field-retype = Zmień
cms-field-time = Z godziną
cms-field-format = Format
cms-field-choices = Opcje
cms-field-choice-add = Dodaj opcję…
cms-field-multiple = Kilka opcji
cms-field-collection = Kolekcja
cms-field-ref-label = Wyświetlana etykieta
cms-field-pattern = Wzorzec
cms-field-pattern-help = Wyrażenie regularne, do którego musi pasować wartość, na przykład ^[a-z0-9-]+$
cms-field-pattern-invalid = Nieprawidłowe wyrażenie
cms-field-subfields = Pola grupy
cms-field-blocks = Typy bloków
type-string = Tekst
type-text = Długi tekst
type-rich-text = Tekst sformatowany
type-image = Obraz
type-file = Plik
type-date = Data
type-boolean = Tak / nie
type-number = Liczba
type-select = Wybór
type-reference = Odwołanie
type-code = Kod
type-object = Grupa pól
type-block = Bloki
type-uuid = Identyfikator
cms-collection-name = Nazwa kolekcji
cms-new-title = Tytuł
cms-new-slug = Adres strony
cms-new-slug-help = Część adresu specyficzna dla każdego wpisu; nie zmieniaj jej po publikacji (SEO).
editor-view = Widok
editor-rulers = Linijki
key-rulers = Ctrl+R
editor-guides = Prowadnice kolumn
editor-guides-count = Kolumny
editor-guides-gutter = Odstęp (px)
editor-guides-margin = Marginesy (px)
editor-guides-max = Maks. szerokość (px)
editor-guides-max-help = Maksymalna szerokość treści; puste: pełna szerokość
key-up = Alt+↑
key-down = Alt+↓
dock-dock = Zadokuj
dock-close-short = Zamknij
cms-entry-open = Otwórz
editor-bp-kind-max-help = Style stosowane na ekranach węższych niż ta szerokość
editor-bp-kind-min-help = Style stosowane na ekranach szerszych niż ta szerokość
editor-bp-width-help = Szerokość (px): za nią podążają style tego punktu przerwania. Pozostaje między sąsiednimi punktami przerwania.
preview-options = Opcje podglądu
preview-here = W przestrzeni roboczej
preview-window = W nowym oknie
key-preview-window = Shift+kliknięcie
preview-tablet = Tablet
preview-mobile = Telefon
preview-reload = Odśwież
preview-close = Zamknij
preview-close-help = Powrót do bieżącej przestrzeni (Esc)
inspector-locale-help = Tłumaczenie: tekst oryginalny jest pokazany na szaro
pages-seo-locale = Przetłumaczony tytuł i opis; adres, obraz i indeksowanie są wspólne dla wszystkich języków
pages-seo-locale-data = SEO odczytywane z pliku danych: przetłumacz je w Lokalizacja › Dane
cms-translated-help = Przetłumaczone na ten język

# Réglages lisibles (3.12)
seo-readable = Tytuł i udostępnianie
seo-readable-help = Ustawienia wyprowadzone z szablonów witryny: każdy wybór przepisuje odpowiednie obliczenie (z możliwością cofnięcia)
seo-tab-title = Tytuł karty
seo-tab-title-help = Tytuł pokazywany na karcie przeglądarki i w wynikach wyszukiwania; strona bez tytułu pokazuje nazwę witryny
seo-title-first = Strona | Witryna
seo-site-first = Witryna | Strona
seo-title-only = Tylko strona
seo-title-sep = Separator
seo-title-sample = Tytuł strony
seo-title-sample-help = Przykładowy tytuł karty
seo-share-default = Domyślny obraz do udostępniania
seo-canonical-base = Adres oficjalny
seo-canonical-base-help = Adres, od którego zaczynają się linki kanoniczne i obrazy do udostępniania stron (https://…)
seo-canonical-invalid = Nieprawidłowy adres (oczekiwano: https://…)
settings-menus = Listy linków szablonów
settings-navigation = Nawigacja
menus-help = Listy linków szablonów (menu witryny): tekst, adres i kolejność każdego linku
menus-none = Nie znaleziono listy linków w szablonach witryny
menus-label = Tekst
menus-url = Adres
menus-new = Nowy link
menus-add = Dodaj link
picks = Wyróżnione
picks-help = Wpisy wybrane z kolekcji i pokazywane przez stronę, w kolejności (identyfikatory zapisane w danych witryny)
picks-missing = Żaden wpis nie ma identyfikatora „{ $slug }”
picks-add = Dodaj wpis…

# Settings › AI assistants (4.2)
settings-mcp = Asystenci AI
settings-mcp-help = Claude, ChatGPT, Codex lub model lokalny odczytują i edytują witrynę otwartą w Revel tymi samymi akcjami co Ty: każda zmiana pojawia się natychmiast i jest cofana za pomocą Ctrl+Z.
mcp-server = Serwer lokalny
mcp-server-help = Połączenie dla asystentów zainstalowanych na tym komputerze (tylko 127.0.0.1, wymagany klucz dostępu). Wyłączony: żaden asystent nie ma dostępu do Revel.
mcp-state-on = Włączony · { $addr }
mcp-state-off = Wyłączony
mcp-state-port-busy = Port jest już używany
mcp-state-failed = Nie udało się uruchomić
mcp-port = Port
mcp-port-help = Lokalny port serwera (od 1024 do 65535); zmieniaj go tylko wtedy, gdy używa go już inny program
mcp-key = Klucz dostępu
mcp-key-help = Wymagany przez serwer. Asystenci uruchamiani poleceniem Revel odczytują go samodzielnie; wkleja się go tylko w Open WebUI.
mcp-key-show = Pokaż klucz
mcp-key-hide = Ukryj klucz
mcp-key-copy = Kopiuj klucz
mcp-key-renew = Odnów
mcp-key-renew-help = Nowy klucz: stary przestaje działać (zastąp go w Open WebUI)
mcp-key-renew-title = Odnowić klucz dostępu?
mcp-connect = Połącz asystenta
mcp-connect-off = Najpierw włącz serwer lokalny
mcp-where-claude-desktop = Claude › Settings › Developer › Edit Config
mcp-where-claude-code = Terminal
mcp-where-lm-studio = LM Studio › Program › Install › Edit mcp.json
mcp-where-open-webui = Open WebUI › Admin Settings › Integrations › External Tool Servers › Add Connection
mcp-how-claude-desktop = Otwórz plik z menu Claude na pasku menu, wklej konfigurację (dodaj tylko „revel” wewnątrz „mcpServers”, jeśli są tam już serwery), zapisz, a następnie zamknij i ponownie otwórz Claude.
mcp-how-claude-code = Uruchom polecenie raz w terminalu: Revel będzie wtedy dostępny w każdym projekcie.
mcp-how-codex = Dodaj na końcu pliku (utwórz go, jeśli go brak), a następnie uruchom ponownie: plik jest wspólny dla aplikacji ChatGPT, wiersza poleceń Codex i rozszerzenia edytora kodu.
mcp-how-lm-studio = Wklej do mcp.json (dodaj tylko „revel” wewnątrz „mcpServers”, jeśli są tam już serwery), a następnie zapisz.
mcp-how-open-webui = Open WebUI 0.6.31 lub nowszy: skopiuj typ, adres, uwierzytelnianie i klucz do nowego połączenia. Open WebUI zainstalowane w Dockerze: adres Docker.
mcp-openapi-help = Opis OpenAPI tych samych akcji, dla narzędzi bez MCP (skrypty, automatyzacje, serwery narzędzi OpenAPI): POST /api/<tool>, uwierzytelnianie Bearer z kluczem dostępu.
mcp-field-type = Typ
mcp-field-url = Adres
mcp-field-url-docker = Adres (Docker)
mcp-field-auth = Uwierzytelnianie
mcp-field-key = Klucz
mcp-activity = Ostatnie akcje
mcp-activity-empty = Brak akcji
mcp-client-unknown = Asystent
mcp-tool-set_text = Zmieniono tekst
mcp-tool-set_attributes = Zmieniono ustawienia elementu
mcp-tool-set_styles = Zmieniono style
mcp-tool-insert_html = Dodano elementy
mcp-tool-remove_node = Usunięto element
mcp-tool-set_data = Zmieniono dane
mcp-tool-set_entry_field = Zmieniono treść CMS
mcp-tool-apply_commands = Zmiany
mcp-tool-undo = Cofnij
mcp-tool-redo = Ponów
action-copy = Kopiuj

# Bouton « Publier » de la barre supérieure (5.1)
pub-panel = Publikacja
pub-public = Witryna publiczna
pub-test = Wersja testowa
pub-never = Nigdy nie opublikowano
pub-at-now = Opublikowano przed chwilą
pub-at-min = Opublikowano { $n } min temu
pub-at-hours = Opublikowano { $n } godz. temu
pub-at-date = Opublikowano { $date }
pub-changes = { $n ->
    [one] 1 strona zmieniona
    [few] { $n } strony zmienione
    [many] { $n } stron zmienionych
   *[other] { $n } strony zmienione
}
pub-files-changed = Zmieniono style lub multimedia
pub-up-to-date = Aktualne
pub-kind-added = Nowe
pub-kind-changed = Zmienione
pub-kind-removed = Usunięte
pub-files = Style, multimedia i skrypty
pub-live = Online
pub-waiting = Oczekuje
pub-needs-access = Wymagany dostęp
pub-hosting = Hosting
pub-hosting-help = Zarządzaj hostingiem witryny (Ustawienia › Publikacja)
pub-empty = Jeszcze nie online
pub-setup = Uruchom witrynę
pub-dirty = Nieopublikowane zmiany
pub-page-open = Otwórz w edytorze
pub-select = Uwzględnij w tej publikacji

# Guided go-live (5.2, 5.3)
setup-title = Uruchom witrynę
setup-host = Dostawca hostingu
setup-host-pick = Wybierz dostawcę
setup-other = Inny
setup-other-help = Inny dostawca z dostępem SFTP
setup-console = Panel klienta
setup-paste = Wiadomość od Twojego hostingu
setup-paste-help = Wklej e-mail z konfiguracją lub stronę z danymi dostępu: serwer, nazwa użytkownika i hasło zostaną rozpoznane
setup-paste-placeholder = Wklej tutaj (opcjonalnie)
setup-env-help = Witryna publiczna: adres Twojej witryny. Wersja testowa: oddzielny adres do sprawdzenia przed publikacją witryny publicznej
setup-advanced = Ustawienia zaawansowane
setup-folder = Folder witryny
setup-connect = Połącz
setup-connecting = Łączenie z serwerem…
setup-connected = Połączono
host-ovh-sftp = Włącz SFTP: Web Cloud › Hosting plans › Twój plan › FTP - SSH, kolumna „SFTP”
host-ovh-access = Dostęp: ta sama karta, „FTP and SFTP server” i „Main login”
host-o2switch-ip = Zezwól temu komputerowi: cPanel › Outils › „Autorisation SSH” › „Ajouter une exception”, port 22
host-o2switch-access = Dostęp: nazwa użytkownika i hasło cPanel (e-mail „Bienvenue chez o2switch”)
host-infomaniak-access = Dostęp: Manager › hosting & website management › Twój hosting › FTP / SSH
host-hostinger-ssh = Włącz SSH: hPanel › Websites › Dashboard › SSH Access › Enable
host-hostinger-access = Dostęp: ta sama sekcja (IP FTP, port 65002, nazwa użytkownika); hasło FTP
host-ionos-create = Utwórz dostęp: Menu › Hosting › SFTP & SSH › Set up
host-ionos-access = Dostęp: SFTP & SSH › Manage › Twoja nazwa użytkownika

# Pre-publish checks (5.4)
check-title = Przed publikacją
check-broken-link = { $n ->
    [one] 1 uszkodzony link
    [few] { $n } uszkodzone linki
    [many] { $n } uszkodzonych linków
   *[other] { $n } uszkodzonego linku
}
check-heavy-image = { $n ->
    [one] 1 obraz powyżej 1 MB
    [few] { $n } obrazy powyżej 1 MB
    [many] { $n } obrazów powyżej 1 MB
   *[other] { $n } obrazu powyżej 1 MB
}
check-missing-alt = { $n ->
    [one] 1 obraz bez tekstu alternatywnego
    [few] { $n } obrazy bez tekstu alternatywnego
    [many] { $n } obrazów bez tekstu alternatywnego
   *[other] { $n } obrazu bez tekstu alternatywnego
}
check-no-title = { $n ->
    [one] 1 strona bez tytułu
    [few] { $n } strony bez tytułu
    [many] { $n } stron bez tytułu
   *[other] { $n } strony bez tytułu
}
publish-anyway = Opublikuj mimo to

# Back to the previous publish (5.5)
pub-rollback-help = Wróć do wersji opublikowanej { $date }
pub-rollback-title = Przywrócić online wersję z { $date }?
pub-rollback-confirm = Wróć
publish-restoring = Przygotowywanie poprzedniej wersji…

# Domain name (5.7)
domain-title = Nazwa domeny
domain-name = Domena
domain-records = Rekordy do utworzenia
domain-records-help = U rejestratora domeny (strefa DNS): rekordy A dla domeny, CNAME dla www
domain-type = Typ
domain-host = Nazwa
domain-value = Wartość
domain-ipv6 = IPv6 (opcjonalnie)
domain-other = U rejestratora domeny: rekordy podane przez Twój hosting
domain-check = Sprawdź
domain-check-help = Zmiana domeny może być stosowana nawet do 48 godzin
domain-checking = Sprawdzanie domeny…
domain-none = { $domain } nie wskazuje na żaden adres
domain-elsewhere = { $domain } wskazuje na { $addresses }, a nie na hosting
domain-points = { $domain } wskazuje na { $addresses }
domain-https = HTTPS aktywne
domain-no-https = HTTPS jeszcze nieaktywne
domain-serves = Domena pokazuje Twoją witrynę
domain-other-page = Domena pokazuje inną stronę niż Twoja witryna
host-ovh-domain-1 = OVHcloud Control Panel › Web Cloud › Twój plan hostingowy › dodaj domenę jako domenę zewnętrzną („Mes sites” / Multisite)
host-ovh-domain-2 = U rejestratora domeny: rekordy A i TXT „ovhcontrol” pokazane przez OVHcloud
host-o2switch-domain-1 = cPanel › Domaines › „Domaines configurés”: dodaj domenę
host-o2switch-domain-2 = U rejestratora domeny: serwery nazw ns1.o2switch.net i ns2.o2switch.net lub rekord A na współdzielony adres IP (cPanel)
host-infomaniak-domain-1 = Manager › Twoja witryna › Information: adresy IPv4 i IPv6
host-infomaniak-domain-2 = U rejestratora domeny: rekordy A (i AAAA) na te adresy
host-hostinger-domain-1 = hPanel › Websites: dodaj witrynę, a potem „Check guide”
host-hostinger-domain-2 = U rejestratora domeny: pokazane serwery nazw lub rekordy A dla @ i www na adres IP; DNSSEC wyłączony
host-ionos-domain-1 = Menu › Domains & SSL › skonfiguruj domenę zewnętrzną
host-ionos-domain-2 = U rejestratora domeny: serwery nazw IONOS lub potwierdzający rekord TXT na @

# Forms (5.8)
insert-help-form = Formularz kontaktowy: wiadomości odbierane e-mailem przez Twój hosting (Ustawienia › Formularze). Przeciągnij na stronę lub kliknij dwukrotnie.
forms-help = Wiadomości z formularzy witryny są wysyłane e-mailem przez Twój hosting (PHP), bez usługi zewnętrznej
forms-recipient = Adres odbiorczy
forms-subject = Temat e-maila
forms-subject-default = Nowa wiadomość od { $site }
forms-success = Komunikat po wysłaniu
forms-success-default = Dziękujemy, Twoja wiadomość została wysłana.
forms-host-yes = { $name }: formularze odbierane
forms-host-no = { $name }: formularze niedostępne (brak PHP)
forms-infomaniak = Infomaniak: włącz funkcję poczty witryny (ustawienia zaawansowane › PHP / Apache) i ustaw e-mail nadawcy; najwyżej 50 wiadomości na 24 godz.
forms-from-domain = Wiadomości są wysyłane z adresu site@twoja-domena: domena musi należeć do hostingu
cms-open-view = Otwórz w CMS
new-site-background = Tło
new-site-dark = Ciemne
new-site-light = Jasne
editor-frame-width = Szerokość strony
editor-frame-width-help = Szerokość strony: { $width } px. Przeciągnij, aby ją zmienić (ekran podąża za nią), dwukrotne kliknięcie: szerokość ekranu

# Component variants (component canvas)
variant-primary = Główny
variant-hover = Najechanie
variant-pressed = Wciśnięty
variant-label = Wariant
variant-new = Nowy wariant
variant-name = Nazwa wariantu
variant-default-name = Wariant { $n }
variant-create = Utwórz
variant-rename = Zmień nazwę wariantu
variant-remove = Usuń wariant
variant-remove-title = Usunąć wariant „{ $name }”?
variant-add = Dodaj wariant
variant-add-hover = Najechanie
variant-add-pressed = Wciśnięty
variant-add-custom = Własny wariant…
variant-add-click = Po kliknięciu…
variant-on-click = Przyjmowany po kliknięciu komponentu
variant-arrow-hover = Po najechaniu
variant-arrow-pressed = Podczas naciskania
variant-arrow-click = Po kliknięciu
variant-arrow-click-help = Kliknięcie komponentu przyjmuje ten wariant, następne wraca do głównego
variant-transition = Przejście
variant-transition-help = Animowana zmiana na warianty najechania i wciśnięcia
variant-transition-none = Bez przejścia
variant-transition-fast = Szybkie · 0,15 s
variant-transition-smooth = Gładkie · 0,3 s
variant-transition-slow = Wolne · 0,6 s
variant-scope = Style wariantu „{ $name }”
component-back-help = Powrót do strony
component-of = Użyty komponent
component-bar = Otwórz komponent

# Text format bar (canvas)
format-bold = Pogrubienie (Ctrl+B)
format-italic = Kursywa (Ctrl+I)
format-link = Link (pusty adres: link usunięty)
format-span = Span: osobno stylizowany tekst (potem zaznacz go w warstwach)
format-url = Adres linku, Enter zatwierdza

# Settings added after the site recreation (D-73)
unit-deg = stopnie
unit-turn = obroty
unit-ms = milisekundy
unit-s = sekundy
unit-percent-self = procent samego elementu
unit-fr = część pozostałego miejsca
unit-min-content = szerokość najdłuższego słowa
unit-max-content = szerokość zawartości w jednym wierszu
unit-min-content-s = Min
unit-max-content-s = Maks.
fx-blur = Rozmycie
fx-brightness = Jasność
fx-contrast = Kontrast
fx-grayscale = Skala szarości
fx-saturate = Nasycenie
fx-sepia = Sepia
fx-invert = Odwrócenie
fx-hue-rotate = Odcień
fx-move-x = Przesuń X
fx-move-y = Przesuń Y
fx-rotate = Obróć
fx-scale = Skaluj
fx-scale-x = Skaluj X
fx-scale-y = Skaluj Y
fx-skew-x = Pochyl X
fx-skew-y = Pochyl Y
fx-inherited = Odziedziczone
fx-remove = Usuń
fx-transform-add = Dodaj transformację
fx-transform-none = Brak transformacji
fx-filter-add = Dodaj filtr
fx-filter-none = Brak filtra
fx-backdrop-add = Dodaj filtr tła
tr-all = Wszystko
tr-opacity = Krycie
tr-transform = Transformacja
tr-color = Kolor tekstu
tr-background = Kolor tła
tr-border = Kolor obramowania
tr-shadow = Cień
tr-filter = Filtry
tr-width = Szerokość
tr-height = Wysokość
ease-ease = Gładko
ease-in = Przyspieszanie
ease-out = Zwalnianie
ease-in-out = Przyspieszanie, potem zwalnianie
ease-linear = Stałe
ease-expo-out = Mocne zwalnianie
tr-none = Brak przejścia
tr-add = Dodaj przejście
tr-remove = Usuń przejście
tr-property = Co się zmienia
tr-duration = Czas trwania
tr-delay = Opóźnienie
tr-easing = Krzywa
grid-equal = Równe
grid-equal-help = Kolumny o tej samej szerokości
grid-custom = Własne
grid-custom-help = Szerokość każdej kolumny (fr: część pozostałego miejsca)
grid-auto = Auto
grid-auto-help = Tyle kolumn, ile się zmieści, z minimalną szerokością
grid-track = Szerokość kolumny
grid-track-add = Dodaj kolumnę
grid-auto-min = Minimalna szerokość kolumny
grid-auto-fit = Rozciągnij
grid-auto-fit-help = Kolumny rozciągają się, gdy jest mniej elementów niż miejsc
styles-row-gap = Odstęp między wierszami
styles-column-gap = Odstęp między kolumnami
gap-link = Ten sam odstęp dla wierszy i kolumn
self-auto = Jak pozostałe
styles-grid-span = Komórki
grid-span-columns = Zajmowane kolumny
grid-span-rows = Zajmowane wiersze
styles-self = Wyrównanie
styles-order = Kolejność
ws-normal = Automatyczne
ws-nowrap = Nigdy (jeden wiersz)
ws-pre-line = Zachowaj wpisane podziały wierszy
ws-pre-wrap = Zachowaj spacje i podziały wierszy
styles-white-space = Zawijanie wierszy
tw-wrap = Normalne
tw-balance = Wyrównane wiersze
tw-pretty = Bez osieroconego ostatniego słowa
styles-text-wrap = Łamanie
styles-overflow-wrap = Długie słowa
ow-normal = Wystają
ow-anywhere = Dzielone, by się zmieściły
styles-word-break = Łamanie słów
wb-normal = Między słowami
wb-break-all = W dowolnym miejscu
wb-keep-all = Nigdy wewnątrz słowa
styles-blend = Mieszanie
styles-transform = Transformacje
styles-transform-origin = Punkt początkowy
styles-clip-path = Przycięcie
styles-text-shadow = Cień tekstu
styles-list-style = Punktory
clip-none = Brak
clip-circle = Koło
clip-ellipse = Elipsa
clip-inset = Prostokąt z zaokrągleniem
clip-triangle = Trójkąt
clip-rhombus = Romb
clip-parallelogram = Równoległobok
clip-arrow = Strzałka
clip-slant = Skośny dół
clip-custom = Własny wielokąt
clip-x = Położenie punktu w poziomie
clip-y = Położenie punktu w pionie
clip-remove = Usuń ten punkt
clip-add = Dodaj punkt
styles-transition = Przejścia
styles-filter = Filtry
styles-backdrop = Filtry tła
blend-normal = Normalny
blend-multiply = Mnożenie
blend-screen = Ekran
blend-overlay = Nakładka
blend-darken = Przyciemnienie
blend-lighten = Rozjaśnienie
blend-color-dodge = Rozjaśnienie koloru
blend-color-burn = Wypalenie koloru
blend-hard-light = Ostre światło
blend-soft-light = Miękkie światło
blend-difference = Różnica
blend-exclusion = Wykluczenie
blend-hue = Odcień
blend-saturation = Nasycenie
blend-color = Kolor
blend-luminosity = Luminancja

block-bound-bg = Obraz tła
block-bound-date = Format daty
block-bound-class = Dodana klasa
block-bound-class-help = Klasa dodawana do elementu, gdy pole jest wypełnione (z własnym stylem)
block-bound-class-name = Klasa
block-bound-fallback = W przeciwnym razie
block-bound-fallback-help = Pole pokazywane, gdy pierwsze jest puste
block-bound-date-raw = Jak zapisano
block-bound-before = Tekst przed
block-bound-after = Tekst po
block-bound-count = Liczba „{ $field }”
block-bound-value-class = Klasa z wartości
block-bound-value-class-help = Klasa złożona z wpisanego początku i wartości pola (block--full, block--column): jedna reguła stylu na wartość w Stylach.
block-bound-value-class-none = Brak
block-bound-value-class-prefix = Początek klasy
block-bound-value-class-default = Jeśli pole jest puste
block-bound-value-class-add = Klasa z wartości pola
block-bound-emphasis = Wyróżnij *słowa*
block-bound-emphasis-help = Słowa wpisane między gwiazdkami w polu („nasza *wiedza*”) otrzymują klasę (kolor, grubość) ustawioną w Stylach.
block-bound-emphasis-class = Klasa wyróżnionych słów
block-bound-visibility = Wyświetlanie
cond-set = jest ustawione
cond-empty = jest puste
cond-equals = jest równe
cond-differs = różni się od
cond-contains = zawiera
cond-more = ma więcej niż (wartości)
cond-less = ma mniej niż (wartości)
cond-field = Testowane pole
cond-test = Test
cond-value = Wartość
cond-count = Liczba wartości
cond-remove = Usuń ten warunek
cond-add = Dodaj warunek
cond-mode = Spełnione warunki
cond-mode-all = Wszystkie te warunki
cond-mode-any = Co najmniej jeden z tych warunków
cond-class-add = Klasa przy warunku
cond-class-remove = Usuń tę klasę
cond-always = Zawsze wyświetlane
cond-raw-help = Warunek zapisany w kodzie: zastąp go regulowanymi warunkami
cond-replace = Zastąp regulowanym warunkiem
block-bound-always = Zawsze
block-bound-if-set = Jeśli „{ $field }” jest ustawione
block-bound-if-empty = Jeśli „{ $field }” jest puste
block-list-exclude = Bez wyświetlanego wpisu
block-list-exclude-help = Na stronie wpisu lista go nie pokazuje („inne projekty”); wpis umieszczony na początku pozostaje widoczny, nawet gdy jest wyświetlanym wpisem.
cms-collection-folder = Folder treści
cms-collection-folder-new = Nowy folder
cms-collection-folder-existing = { $folder } (wpisy: { $count })
block-list-source = Źródło
block-list-of-field = „{ $field }” tego wpisu
block-bound-value = Wartość
insert-help-lottie = Animacja wyeksportowana z After Effects (Bodymovin) lub LottieFiles, odtwarzana przez witrynę bez usługi zewnętrznej
block-lottie = Animacja Lottie
block-lottie-file = Plik
block-lottie-none = Brak pliku
block-lottie-choose = Wybierz animację
block-lottie-play = Odtwarzanie
lottie-play-auto = Na ekranie
lottie-play-auto-help = Odtwarzana, gdy jest widoczna, w przeciwnym razie wstrzymana
lottie-play-hover = Po najechaniu
lottie-play-hover-help = Odtwarzana do przodu po najechaniu, a potem wstecz
lottie-play-click = Po naciśnięciu
lottie-play-click-help = Odtwarzana ponownie przy każdym naciśnięciu
lottie-play-scroll = Przy przewijaniu
lottie-play-scroll-help = Podąża za przewijaniem strony
block-lottie-speed = Prędkość
block-lottie-loop = Zapętlenie
act-title = Akcje
act-help = Naciśnięcie, najechanie lub przewijanie działa na ten element lub inny (menu, nagłówek, baner)
act-add = Dodaj akcję
act-remove = Usuń akcję
act-trigger = Wyzwalacz
act-trigger-click = Po naciśnięciu
act-trigger-hover = Po najechaniu
act-trigger-scroll = Przy przewijaniu
act-trigger-view = Na ekranie
act-trigger-load = Przy ładowaniu
act-scroll-px = Po (przewiniętych pikselach)
act-action = Akcja
act-class = Dodaj klasę
act-class-help = Klasa jest dodawana i usuwana przy następnym naciśnięciu (lub po zakończeniu najechania, a przy przewijaniu wstecz)
act-toggle = Pokaż lub ukryj
act-toggle-help = Widoczny, jeśli ukryty, ukryty, jeśli widoczny
act-show = Pokaż
act-show-help = Element docelowy się pojawia
act-hide = Ukryj
act-hide-help = Element docelowy znika
act-play = Odtwórz lub wstrzymaj
act-play-help = Wideo elementu docelowego jest odtwarzane lub wstrzymywane
act-dismiss = Zamknij na stałe
act-dismiss-help = Ukryty i nie pokazywany odwiedzającemu ponownie (baner zgody)
act-random = Losowy element podrzędny
act-random-help = Pokazuje jeden element podrzędny celu, wybrany losowo (mantra, cytat), pozostałe są ukryte
act-random-delay = Opóźnienie (ms)
act-spin = Obrót
act-spin-help = Za każdym razem dodaje obrót do celu, plus losowy ćwierć obrotu (kompas, koło)
act-spin-deg = Kąt (stopnie)
act-sound = Odtwórz dźwięk
act-sound-help = Odtwarza dźwięk raz (wybierany losowo, gdy jest ich kilka); wyciszany przełącznikiem „Dźwięki”
act-music = Włącz muzykę
act-music-help = Odtwarza muzykę w pętli, wybraną losowo spośród wybranych; zastępuje poprzednią; wyciszana przełącznikiem „Muzyka”
act-sound-add = Dodaj plik dźwiękowy
act-sound-remove = Usuń ten plik
act-sound-volume = Głośność (%)
act-class-invalid = Nazwa klasy: tylko litery, cyfry, myślniki i podkreślenia
act-class-name = Klasa
act-target = Cel
act-target-self = Ten element
act-target-parent = Element nadrzędny
act-target-next = Następny element
act-target-body = Cała strona
act-target-id = Element „{ $id }”
act-target-class = Klasa „{ $class }”
act-target-closest = Najbliższy otaczający „{ $class }”
insert-tabs = Karty
insert-help-tabs = Jeden przycisk na panel; wybrany panel jest wyświetlany w witrynie
insert-text-tabs = Karta
insert-group-interactive = Interaktywne
insert-navbar = Pasek nawigacji
insert-help-navbar = Marka i linki; na tablecie i telefonie przycisk Menu otwiera linki
insert-text-navbar = Marka|Menu|Start|O nas|Kontakt
insert-ticker = Taśma przewijana
insert-help-ticker = Jej elementy przewijają się bez końca (logo, zespół, słowa); prędkość i kierunek na górze Stylów
insert-text-ticker = Element
insert-carousel = Karuzela
insert-help-carousel = Slajdy obok siebie, strzałki i licznik; przesunięcie palcem lub przewijanie
insert-slideshow = Pokaz slajdów
insert-help-slideshow = Jeden slajd naraz, autoodtwarzanie, kropki
insert-text-slider = Slajd|Poprzedni|Następny
insert-consent = Zgoda na pliki cookie
insert-help-consent = Baner wyświetlany, dopóki odwiedzający nie dokona wyboru; Google Analytics ładowane po wyrażeniu zgody (identyfikator na górze Stylów)
insert-text-consent = Za Twoją zgodą mierzymy oglądalność witryny.|Odrzuć|Akceptuj
insert-switch = Przełącznik dźwięku
insert-help-switch = Przycisk „Wł. / Wył.” wyciszający muzykę lub dźwięki witryny (ustawienie na górze Stylów)
insert-text-switch = Muzyka|Wł.|Wył.
insert-theme-switch = Przełącznik jasny / ciemny
insert-help-theme-switch = Przycisk przełączający witrynę między jasnymi i ciemnymi kolorami (Ustawienia › Kolory); przeglądarka zapamiętuje wybór
insert-text-theme-switch = Motyw|Ciemny|Jasny
insert-filters = Filtry listy
insert-help-filters = Przyciski filtrujące listę strony (projekty danej specjalizacji); oznacz jej elementy jako „Filtrowany element” na górze Stylów
insert-text-filters = Wszystkie|Opcja 1|Opcja 2
insert-video-player = Odtwarzacz wideo
insert-help-video-player = YouTube, Vimeo lub wideo witryny, uruchamiane naciśnięciem obrazu okładki; adres na górze Stylów
insert-text-video-player = Odtwórz wideo
ticker-title = Taśma przewijana
items-title = Elementy
items-replace = Zastąp obraz
items-up = Przesuń w górę
items-down = Przesuń w dół
items-add-image = Dodaj obraz
items-item = Element { $n }
items-tabs = Karty
items-tab = Karta { $n }
items-add-tab = Dodaj kartę
items-links = Linki
items-add-link = Dodaj link
reveal-mode = Odsłanianie
reveal-fade = Przenikanie
reveal-fade-help = Górny obraz pojawia się w całości po najechaniu
reveal-wipe = Przesłona
reveal-wipe-help = Górny obraz jest widoczny do wskaźnika, jak przesłona podążająca za nim
reveal-spot = Reflektor
reveal-spot-help = Górny obraz jest widoczny w kole wokół wskaźnika
reveal-radius = Promień
reveal-radius-help = Rozmiar reflektora wokół wskaźnika
reveal-soft = Miękka krawędź
reveal-soft-help = Szerokość zanikania na krawędzi reflektora
reveal-under = Obraz dolny
reveal-over = Obraz odsłaniany
icons-arrows = Strzałki
icons-interface = Interfejs
icons-contact = Kontakt
icons-media = Multimedia
icons-objects = Obiekty
icons-design = Projektowanie
icons-custom = Zaimportowane
icons-import = Importuj ikony SVG
accordion-open = Otwarty przy ładowaniu
accordion-open-help = Panel otwarty po załadowaniu strony
accordion-single = Jeden otwarty naraz
accordion-single-help = Otwarcie panelu zamyka sąsiednie (Chrome i Edge 120, Firefox 130, Safari 17.2; gdzie indziej każdy otwiera się sam)
accordion-add = Dodaj panel
ticker-speed = Prędkość
ticker-fade = Zanikające krawędzie
anim-speed = Prędkość
anim-speed-help = Piksele przebyte w ciągu sekundy
anim-fade = Zanikanie
anim-fade-help = Zanikanie na obu krawędziach (szerokość)
anim-dir-up = W górę
anim-dir-down = W dół
filters-role = Filtry listy
filters-role-help = Przyciski pokazujące tylko te elementy listy, których tagi zawierają ich wartość (projekty danej specjalizacji); bez JavaScript wszystko pozostaje widoczne.
filters-role-none = Brak roli
filters-role-group = Grupa filtrów
filters-role-button = Przycisk filtra
filters-role-item = Filtrowany element
filters-role-empty = Komunikat „Brak wyników”
filters-param = Parametr adresu
filters-param-help = Wybrany filtr jest zachowywany w adresie (?expertise=…): link może otworzyć listę już przefiltrowaną. Puste: brak adresu.
filters-value = Wartość
filters-value-help = Wartość wyszukiwana w tagach elementów; puste: pokaż wszystko.
filters-value-written = Wpisana
filters-value-all = Puste: pokaż wszystko
filters-tags = Tagi
filters-tags-help = Pole listy wpisu (specjalizacje, kategorie), którego wartości wyszukują przyciski.
consent-role = Zgoda na pliki cookie
consent-role-help = Baner wyświetlany, dopóki odwiedzający nie dokona wyboru; pomiar oglądalności ładuje się dopiero po wyrażeniu zgody. Wybór jest zapamiętywany w przeglądarce.
consent-role-none = Brak roli
consent-role-banner = Baner
consent-role-accept = Przycisk „Akceptuj”
consent-role-refuse = Przycisk „Odrzuć”
consent-role-open = Link „Zarządzaj plikami cookie”
consent-ga = Google Analytics 4
consent-ga-help = Identyfikator pomiaru (G-…) ładowany po wyrażeniu zgody; puste: brak pomiaru.
tabs-title = Karty
tabs-hover = Po najechaniu
tabs-hover-help = Karta zmienia się po najechaniu i fokusie (lista sterująca obrazem tła lub wideo); w przeciwnym razie po naciśnięciu. Wideo z autoodtwarzaniem w pokazanym panelu się uruchamiają.
tabs-keys = Klawisze klawiatury
tabs-keys-help = Klawisze strzałek oraz W, A, S, D zmieniają karty w dowolnym miejscu strony (menu gry, pokaz slajdów)
tabs-sound = Dźwięk przy zmianie
tabs-sound-help = Odtwarzany za każdym razem, gdy aktywna staje się inna karta; wyciszany przełącznikiem „Dźwięki”
switch-role = Przełącznik
switch-role-help = Włącza lub wyłącza muzykę albo dźwięki witryny (domyślnie włączone, wybór zapamiętuje przeglądarka odwiedzającego); stan „Wył.” stylizuje się w Stylach
switch-none = Brak
switch-music = Muzyka
switch-sounds = Dźwięki
switch-theme = Motyw jasny / ciemny
switch-text-on = Tekst, gdy włączony
switch-text-off = Tekst, gdy wyłączony
switch-text-add = Pokaż tutaj stan (Wł. / Wył.)
switch-text-help = Tekst tego elementu podąża za stanem zawierającego go przełącznika
switch-text-remove = Przestań pokazywać stan
tabs-role = W kartach
tabs-role-help = Karta: przycisk lub link pokazujący swój panel (n-ta karta pokazuje n-ty panel). Panel: treść pokazywana przez swoją kartę.
tabs-role-none = Zwykły element
tabs-role-tab = Karta
tabs-role-panel = Panel
slider-title = Slajdy
slider-autoplay = Autoodtwarzanie
slider-autoplay-help = Sekundy między slajdami; puste: ręcznie
slider-loop = Zapętlenie
insert-accordion = Akordeon
insert-help-accordion = Tytuł otwierający lub zamykający swoją treść, bez skryptu
insert-text-accordion = Pytanie
settings-transition = Przejście między stronami
settings-transition-help = Przejście z jednej strony witryny na drugą; bez efektu w Firefox, wyłączone dla odwiedzających, którzy proszą o mniej ruchu
settings-smooth-scroll = Płynne przewijanie
settings-smooth-scroll-help = Kółko myszy przesuwa płynnie stronę lub przewijaną kolumnę pod wskaźnikiem zamiast skakać o jedną działkę; wyłączone dla odwiedzających, którzy proszą o mniej ruchu
settings-feeds = Kanały RSS
settings-feeds-help = Jeden kanał na kolekcję, odświeżany przy każdej kompilacji; czytniki kanałów znajdują tam najnowsze wpisy
settings-feeds-none = Brak kolekcji ze stroną dla każdego wpisu: kanał RSS wymienia wpisy, które mają własną stronę.
settings-feed-path = Adres kanału
settings-feed-limit = Liczba wpisów
settings-feed-title = Tytuł
settings-feed-description = Opis
settings-feed-site = Jak w witrynie
settings-feed-site-desc = Jak w witrynie
transition-none = Brak
transition-fade = Przenikanie
transition-slide = Przesuwanie
transition-slide-up = Przesuń w górę
transition-push = Wypchnij
transition-zoom = Powiększenie
transition-blur = Rozmycie
transition-wipe = Wymazanie
transition-curtain = Kurtyna
transition-flip = Obrót
help-menu = Pomoc i skróty
help-title = Pierwsze kroki
block-list-computed-short = Lista obliczana
cms-sub-locked-short = Zdefiniowane w .pages.yml
locales-no-url-short = Brak linków między językami: brak adresu witryny
update-title = Aktualizacje
update-help = Nowe wersje Revel, sprawdzane po podpisie przed zainstalowaniem
update-version = Zainstalowana wersja
update-check = Sprawdź aktualizacje
update-install = Zainstaluj i uruchom ponownie
update-available = Dostępna wersja { $version }
update-none = Revel jest aktualny
update-disabled = Aktualizacje nie są dostępne dla tej kompilacji Revel
update-pill = Wersja { $version }
update-pill-help = Dostępna jest nowsza wersja Revel: zainstaluj ją w ustawieniach
update-prereleases = Otrzymuj wersje wstępne
update-prereleases-help = Wersje testowe publikowane przed wersją stabilną są również oferowane; mogą zawierać błędy
update-available-pre = Dostępna wersja wstępna { $version }
update-pill-pre = Wersja wstępna { $version }
styles-truncate = Obcinanie
styles-truncate-help = Tekst w jednym wierszu, kończący się „…” w razie przepełnienia
error-not-a-site = Ten folder nie zawiera witryny Revel.
error-trash = Nie udało się przenieść folderu do kosza: { $detail }
site-delete = Usuń witrynę…
site-delete-title = Usunąć „{ $name }”?
site-delete-body = Folder witryny trafia do kosza komputera; można go stamtąd przywrócić.
site-delete-confirm = Przenieś do kosza
site-thumb = Miniatura
site-thumb-color = Kolor
site-thumb-homepage = Strona główna
site-thumb-auto = Według nazwy witryny
site-thumb-custom = Inny kolor
site-thumb-retake = Odśwież
site-thumb-retake-help = Zrób ponownie obraz strony głównej (dzieje się też samo po każdej zmianie witryny)
site-thumb-taking = Robienie obrazu…
nav-back-to-site = Wróć do { $name }
settings-translate-more = Więcej języków ({ $count })
settings-translate-less = Mniej języków
settings-translate-none = Brak zainstalowanego języka
styles-grid = Grid
grid-columns-count = Kolumny
grid-rows-count = Wiersze
grid-edit = Szerokość każdej kolumny
grid-flow-row = Wypełniaj wierszami
grid-flow-column = Wypełniaj kolumnami
grid-dense = Wypełniaj puste komórki
grid-masonry = Masonry
grid-masonry-help = Masonry: każdy element trafia pod najkrótszą kolumnę, bez pustych wierszy (różne wysokości, jak Masonry we Framer)
grid-masonry-order = Kolumny po kolei
grid-masonry-order-help = Element 1 w pierwszej kolumnie, 2 w drugiej i tak dalej (w przeciwnym razie: w najkrótszej kolumnie)
gap-columns = Kolumny
gap-rows = Wiersze
align-more = Więcej opcji wyrównania
align-content-columns = Kolumny
align-content-rows = Wiersze
styles-grid-child = Element potomny grid
grid-parent-edit = Edytuj nadrzędny grid
flex-parent-edit = Edytuj nadrzędny flex
child-position = Pozycja
child-auto = Auto
child-auto-help = Umieszczony po pozostałych, na wybranej liczbie komórek
child-manual = Ręcznie
child-manual-help = Umieszczony między wybranymi liniami siatki
grid-column-line = Linia kolumny
grid-row-line = Linia wiersza
grid-column-start-end = Kolumna: początek, koniec
grid-row-start-end = Wiersz: początek, koniec
styles-align-self = Wyrównanie
styles-justify-self = Rozmieszczenie
flex-sizing = Rozmiarowanie
order-auto = Brak
order-first = Pierwszy
order-last = Ostatni
order-custom = Inna kolejność (liczba)

# Importing a site (step 7)
import-title = Importuj witrynę
import-source = Źródło
import-source-folder = Folder
import-source-folder-help = Folder ze stronami HTML, eksport Webflow (rozpakowany) lub projekt Eleventy na tym komputerze; dodawany jest do niego tylko plik site.revel.json (oraz treści CMS Webflow)
import-source-git = Repozytorium Git
import-source-git-help = Kopiuje repozytorium GitHub, dowolne repozytorium Git przez HTTPS lub repozytorium zespołu w folderze współdzielonym do folderu projektów, wraz z historią, a następnie je importuje
import-folder = Folder
import-choose = Wybierz…
import-url = Adres repozytorium
import-url-placeholder = https://github.com/account/site
import-url-help = github.com/account/repository, adres gałęzi lub folderu (…/tree/main/docs), dowolne repozytorium HTTPS lub folder współdzielony zespołu (NAS, sieć lokalna); repozytorium prywatne: połączenie z GitHub w Ustawieniach
import-git-folder = Folder…
import-git-folder-help = Repozytorium zespołu w folderze współdzielonym (NAS, sieć lokalna): Ustawienia › Wersje › Praca zespołowa › Folder współdzielony
import-kind-static = Witryna HTML · { $count } { $count ->
        [one] strona
        [few] strony
        [many] stron
       *[other] strony
    }
import-kind-static-help = Strony, wspólny układ, arkusz stylów w panelu Style, pliki skopiowane bez zmian
import-kind-eleventy = Projekt Eleventy
import-kind-build = Witryna do zbudowania · { $tool }
import-kind-build-help = Ta witryna jest tworzona przez { $tool }: jej strony pochodzą z kompilacji. „Zbuduj w Revel” uruchamia ją na tym komputerze; w przeciwnym razie zaimportuj folder wyjściowy, jeśli repozytorium go ma, albo opublikowaną witrynę
import-kind-unknown = Nic do zaimportowania
import-kind-unknown-help = Brak strony HTML w katalogu głównym i brak projektu Eleventy
import-has-site = Już zaimportowano
import-no-license = Brak licencji
import-no-license-help = Brak pliku licencji w tym repozytorium: witryna pozostaje własnością autora. Sprawdź, czy wolno Ci jej użyć ponownie.
import-output = Importuj „{ $dir }”
import-output-help = Folder ze stronami znaleziony w repozytorium
import-run = Importuj
import-cloning = Kopiowanie repozytorium…
import-importing = Importowanie…
error-import-needs-build = Witryna do zbudowania za pomocą { $tool }: zaimportuj folder utworzony przez jej kompilację (dist, build…).
error-import-nothing = Brak strony HTML lub projektu Eleventy w tym folderze.
error-git-url = Nie rozpoznano adresu repozytorium.
error-git-clone-cancelled = Kopiowanie repozytorium anulowane.
error-not-a-clone = Ten folder nie jest kopią repozytorium w trakcie importu.
import-source-link = Link
import-source-link-help = Opublikowana witryna odczytana spod swojego adresu (Framer, Webflow, dowolna działająca witryna): strony, style, obrazy i czcionki kopiowane do folderu projektów
import-source-figma = Figma
import-source-figma-help = Ramki pliku Figma jako strony: auto layout, tekst, style, obrazy, komponenty
import-source-wordpress = WordPress
import-source-wordpress-help = Plik eksportu WordPress: strony, wpisy, menu i obrazy, w szablonie Blog Revel
import-link = Adres witryny
import-link-placeholder = https://mysite.framer.website
import-link-help = Adres strony głównej; strony wyszukiwane od strony głównej i z mapy witryny, reguły robots.txt są respektowane
import-reading = Odczytywanie witryny…
import-progress = Strony: { $pages } · pliki: { $files } · { $size }
import-made-with = Utworzono za pomocą { $tool }
import-cms = Treść CMS
import-cms-csv = Pliki CSV
import-cms-csv-help = Pliki wyeksportowane z Webflow (panel CMS › kolekcja › Export), jedna kolekcja na plik
import-cms-api = API Webflow
import-cms-api-help = Treść odczytywana online, wraz z wersjami roboczymi, za pomocą tokenu witryny: Webflow › Site settings › Apps & integrations › API access › Generate API token (CMS: read). Token używany tylko do tego importu
import-cms-add = Dodaj…
import-cms-none = Brak pliku
import-cms-token = Token dostępu
import-figma-link = Adres pliku
import-figma-link-placeholder = https://www.figma.com/design/…
import-figma-link-help = Link do pliku skopiowany w Figma (Share › Copy link)
import-figma-token = Token Figma
import-figma-token-help = Figma › Settings › Security › Personal access tokens › Generate new token (File content: read); przechowywany w pęku kluczy komputera
import-figma-token-saved = Token zapisany
import-figma-forget = Zapomnij
import-figma-forget-help = Usuń token Figma z pęku kluczy komputera
import-figma-read = Odczytaj plik
import-figma-reading = Odczytywanie pliku…
import-figma-frames = Ramki
import-figma-frames-help = Każda zaznaczona ramka staje się stroną; pierwsza jest stroną główną
import-figma-all = Wszystkie
import-figma-none = Żadne
import-wp-file = Plik eksportu
import-wp-file-help = Plik .xml utworzony przez WordPress: Tools › Export › All content
import-wp-counts = Strony: { $pages } · wpisy: { $posts } · multimedia: { $media }
import-wp-media = Pobierz obrazy
import-wp-media-help = Obrazy stron i wpisów kopiowane z oryginalnej witryny (online); w przeciwnym razie zachowane są oryginalne adresy
import-downloading = Pobieranie… pliki: { $files }
import-done-title = Witryna zaimportowana
import-stat-pages = Strony
import-stat-layouts = Układy
import-stat-styles = Reguły stylów
import-stat-animations = Animacje
import-stat-forms = Formularze
import-stat-components = Komponenty
import-stat-entries = Wpisy
import-stat-files = Pliki
import-open = Otwórz w edytorze
error-import-url = Nie rozpoznano adresu witryny.
error-import-unreachable = Witryna jest nieosiągalna: { $detail }
error-import-not-html = Ten adres nie prowadzi do strony HTML.
error-import-cancelled = Import anulowany.
error-import-wxr = { $detail }
error-figma-url = Nie rozpoznano adresu pliku Figma.
error-figma-token = Token Figma brakuje lub został odrzucony.
error-figma-not-found = Nie znaleziono pliku Figma lub jest on niedostępny z tym tokenem.
error-figma = Figma: { $detail }
error-webflow-token = Token API Webflow został odrzucony.
import-build = Zbuduj w Revel
import-build-help = Revel pobiera narzędzia i zależności projektu, buduje go na tym komputerze, a następnie importuje utworzone strony
import-build-checking = Sprawdzanie potrzebnych narzędzi…
import-build-missing-ruby = Jekyll wymaga zainstalowanych na komputerze Ruby i Bundler (ruby-lang.org): Revel nie może ich zainstalować samodzielnie
import-build-missing-go = Ta witryna Hugo używa modułów: na komputerze musi być zainstalowany Go (go.dev)
import-build-missing-build-script = Brak skryptu „build” w package.json: nie ma czego budować
import-build-title = Zbuduj witrynę { $tool }
import-build-risk = Uruchamia kod projektu
import-build-risk-help = Budowanie uruchamia na tym komputerze kod projektu i jego zależności, tak jak robi to jego autor: tylko dla repozytorium, któremu ufasz
import-build-system = na komputerze
import-build-ready = już pobrane
import-build-download = do pobrania
import-build-deps = Zależności ({ $manager })
import-build-lock = { $file } zignorowany
import-build-lock-help = Plik innego menedżera pakietów (Bun): npm sam dobiera wersje zależności i mogą się one różnić od tych u autora
import-build-run = Zbuduj
import-build-log = Dziennik
import-build-step-start = Przygotowywanie…
import-build-step-download = Pobieranie { $tool } · { $done } / { $total }
import-build-step-download-start = Pobieranie { $tool }…
import-build-step-install = Instalowanie zależności ({ $manager })…
import-build-step-build = Budowanie witryny ({ $tool })…
import-build-step-serve = Uruchamianie zbudowanej witryny…
import-build-step-read = Odczytywanie stron · strony: { $pages } · pliki: { $files }
import-published = Importuj opublikowaną witrynę · { $host }
import-published-help = Adres znaleziony w plikach projektu lub na jego stronie: działająca witryna jest odczytywana tak jak ze źródła „Link”
error-build-download = Pobieranie nie powiodło się: { $detail }.
error-build-integrity = Pobrany plik został zmieniony, instalacja odrzucona: { $detail }.
error-build-archive = Nieczytelne archiwum: { $detail }.
error-build-platform = Narzędzie niedostępne dla tego komputera.
error-build-needs-ruby = Jekyll wymaga zainstalowanych na komputerze Ruby i Bundler.
error-build-needs-go = Ta witryna Hugo wymaga zainstalowanego na komputerze Go.
error-build-no-script = Brak skryptu „build” w package.json.
error-build-step = Niepowodzenie: { $step ->
    [install] instalowanie zależności
    [build] budowanie
    [serve] uruchamianie zbudowanej witryny
   *[other] etap { $step }
} (kod { $code }).
error-build-timeout = Zatrzymano po przekroczeniu czasu: { $step ->
    [install] instalowanie zależności
    [build] budowanie
    [serve] uruchamianie zbudowanej witryny
   *[other] etap { $step }
}.
error-build-no-output = Kompilacja nie utworzyła żadnych stron.
settings-build-tools = Narzędzia kompilacji
settings-build-tools-help = Narzędzia pobierane przez Revel do budowania zaimportowanych witryn (Node.js, Hugo) oraz pakiety zapisane w pamięci podręcznej do późniejszych kompilacji
build-tools-none = Brak pobranych narzędzi
build-tools-cache = Pakiety w pamięci podręcznej
build-tools-clear = Wyczyść
build-tools-remove = Usuń

# Media on hover (video or image that changes with the hovered item)
insert-hover-media = Multimedia najechanych elementów
insert-reveal = Odsłanianie po najechaniu
drag-title = Przeciąganie
drag-help = Odwiedzający przesuwają ten element myszą lub palcem
drag-axis = Kierunek
drag-axis-free = Dowolny
drag-axis-x = Poziomo
drag-axis-y = Pionowo
drag-bounds = Pozostań w kontenerze
drag-bounds-help = Element nie wychodzi poza element, który go zawiera
drag-momentum = Pęd
drag-momentum-help = Po puszczeniu element sunie dalej i zwalnia
drag-collide = Kolizje
drag-collide-help = Elementy do przeciągania w tym samym kontenerze odpychają się zamiast nachodzić na siebie
insert-help-reveal = Drugi obraz nad pierwszym, odsłaniany po najechaniu: w całości, do wskaźnika lub wokół niego (szkic, potem gotowa praca)
insert-help-hover-media = Wideo pokazujące wideo najechanego elementu: wpisy listy CMS (ich pole „Wideo”) lub wybrane elementy; ustawienie na górze Stylów
insert-progress = Pasek postępu
insert-help-progress = Wypełnia się podczas przewijania zawierającej go kolumny lub strony (ustawienie w inspektorze); kolor wypełnienia: kolor tekstu
insert-clock = Zegar
insert-help-clock = Godzina lub data odwiedzającego w języku strony, odświeżana co sekundę
insert-pixels = Płótno pikseli
insert-help-pixels = Piksele rysowane pod wskaźnikiem: znikający ślad lub trwały rysunek
clock-show = Pokazuje
clock-time = Godzina
clock-date = Data
clock-both = Oba
clock-seconds = Sekundy
clock-12h = Zegar 12-godzinny
clock-weekday = Dzień tygodnia
clock-year = Rok
clock-month = Miesiąc
clock-month-long = Pełna nazwa
clock-month-short = Skrót
clock-month-numeric = Liczba
clock-month-none = Ukryty
pixels-mode = Tryb
pixels-trail = Ślad
pixels-trail-help = Piksele znikają po chwili
pixels-draw = Rysunek
pixels-draw-help = Piksele zostają do ponownego wczytania strony
pixels-size = Rozmiar piksela
pixels-size-help = Bok kwadratu w pikselach strony
pixels-color = Kolor
pixels-click = Tylko przy wciśnięciu
pixels-click-help = Wyłączone: piksele podążają za wskaźnikiem, gdy tylko nad nim przejdzie
pixels-fade = Zanikanie
pixels-fade-help = Czas zniknięcia piksela śladu
inspector-spy = Podąża za widoczną sekcją
inspector-spy-help = Link do czytanej sekcji otrzymuje klasę is-active (stan stylu „Aktywny”); element otrzymuje data-revel-section z identyfikatorem sekcji.
inspector-progress = Podąża za
inspector-progress-help = Przewijanie wypełniające pasek: zawierająca go kolumna (pierwszy przewijany kontener wokół niego) lub cała strona
inspector-progress-parent = Kolumna
inspector-progress-page = Strona
hover-title = Multimedia po najechaniu
hover-help = To wideo (lub obraz) pokazuje multimedia elementu najechanego lub osiągniętego klawiaturą; przy ładowaniu pierwszego. Pokazany element otrzymuje stan „Aktywny”, uruchomione multimedia stan „Multimedia gotowe”.
hover-on = Zmienia się z najechanym elementem
hover-on-image = Zmienia się z najechanym elementem
hover-on-help = Pokazuje wideo najechanego elementu (wpisy listy CMS lub wybrane elementy)
hover-source = Najeżdżane elementy
hover-source-help = Wpisy listy CMS (każdy pokazuje własne multimedia) lub elementy wybierane pojedynczo
hover-source-list = Wpisy „{ $list }”
hover-source-manual = Elementy wybierane pojedynczo
hover-source-manual-help = Zaznacz element na stronie, a potem wybierz te multimedia w „Multimedia po najechaniu › Pokazywane w” w jego ustawieniach.
hover-none = Brak
hover-field-choose = Wybierz pole…
hover-video-field = Wideo każdego wpisu
hover-video-field-help = Pole wpisu podające adres jego wideo (plik .mp4 z multimediów lub bezpośredni link)
hover-image-field = Obraz każdego wpisu
hover-image-field-help = Pole wpisu podające jego obraz
hover-video-field-none = Żadne pole tych wpisów nie zawiera wideo. Dodaj je w CMS: plik lub pole tekstowe z adresem pliku wideo (.mp4, .webm).
hover-image-field-none = Żadne pole tych wpisów nie zawiera obrazu. Dodaj pole obrazu w CMS.
hover-poster-field = Obraz tymczasowy
hover-poster-help = Pokazywany podczas ładowania wideo
hover-items = Zmienia się po najechaniu na
hover-item-in-list = { $item }, w każdym wpisie „{ $list }”
hover-select = Zaznacz
hover-shown-in = Pokazywane w
hover-shown-in-help = Multimedia po najechaniu, które ten element zmienia po najechaniu
hover-missing = Multimedia po najechaniu, które ten element zmieniał, już nie istnieją.
hover-list-note = Każdy wpis listy pokazuje własne multimedia: zmień je w CMS.
hover-video-src = Wideo (adres)
hover-image-src = Obraz (adres)
hover-poster-src = Obraz tymczasowy (adres)
inspector-src-video = Wideo (adres)
inspector-src-audio = Dźwięk (adres)
styles-state-ready = Multimedia gotowe
styles-state-in-paused = Wideo wstrzymane
styles-state-in-playing = W odtwarzanym filmie
styles-state-in-open-menu = Menu otwarte
styles-state-in-empty-cart = Pusty koszyk
tabs-pair-tab = Karta { $n } z { $count }: pokazuje
tabs-pair-panel = Panel { $n } z { $count }: pokazywany przez
tabs-pair-none = Brak panelu: paneli jest mniej niż kart.
tabs-pair-repeated = Karty lub panele powtarzane przez listę: n-ty wpis pokazuje n-ty panel.
tabs-pair-mismatch = Kart: { $tabs }, paneli: { $panels }: n-ta karta pokazuje n-ty panel, użyj tylu samo po każdej stronie.

# Site information (contact details, social links, menus)
siteinfo-rename = Zmień nazwę
siteinfo-rename-help = Kliknij dwukrotnie, aby zmienić nazwę
siteinfo-open-link = Otwórz link
siteinfo-title = Informacje o witrynie
siteinfo-help = Wartości zapisane raz i używane ponownie na stronach: zmienione tutaj, zmieniają się wszędzie. Powiąż tekst, link lub obraz z wartością w Style › Informacje o witrynie.
siteinfo-none = Brak informacji.
siteinfo-label = Wyświetlana nazwa
siteinfo-uses = { $count ->
    [0] Nieużywane
    [one] Użyte raz
    [few] Użyte { $count } razy
    [many] Użyte { $count } razy
   *[other] Użyte { $count } razy
}
siteinfo-uses-help = Liczba miejsc w witrynie (strony, układy, komponenty) używających tej wartości; używanej wartości nie można usunąć
siteinfo-new-label = Nazwa (np. Godziny otwarcia)
siteinfo-new-value = Wartość
siteinfo-add = Dodaj
siteinfo-bind = Informacje o witrynie
siteinfo-bind-help = Tekst, link lub obraz pobrany z wartości witryny (Ustawienia › Informacje o witrynie): zmieniany raz, wszędzie
siteinfo-year = Bieżący rok
siteinfo-link-written = Wpisany ręcznie
siteinfo-link-mail = { $name } (wyślij e-mail)
siteinfo-link-tel = { $name } (zadzwoń)
siteinfo-image-written = Wybrany ręcznie
siteinfo-edit = Edytuj informacje o witrynie
siteinfo-key-name = Nazwa
siteinfo-key-title = Tytuł
siteinfo-key-description = Opis
siteinfo-key-url = Adres witryny
siteinfo-key-locale = Język (region)
siteinfo-key-lang = Język
siteinfo-key-address = Adres
siteinfo-key-street = Ulica
siteinfo-key-postcode = Kod pocztowy
siteinfo-key-city = Miasto
siteinfo-key-country = Kraj
siteinfo-key-phone = Telefon
siteinfo-key-fax = Faks
siteinfo-key-email = E-mail
siteinfo-key-social = Sieci społecznościowe
siteinfo-key-facebook = Facebook
siteinfo-key-linkedin = LinkedIn
siteinfo-key-instagram = Instagram
siteinfo-key-youtube = YouTube
siteinfo-key-twitter = X (Twitter)
siteinfo-key-x = X
siteinfo-key-tiktok = TikTok
siteinfo-key-ga4 = Google Analytics
siteinfo-key-share_image = Obraz do udostępniania
siteinfo-key-menus = Menu
siteinfo-key-contact_cta = Wezwanie do kontaktu
siteinfo-key-subtitle = Podtytuł
siteinfo-key-text = Tekst
siteinfo-key-button = Przycisk
siteinfo-key-contact_form = Formularz kontaktowy
siteinfo-key-recipient = Odbiorca
siteinfo-key-subject = Temat
siteinfo-key-success = Wiadomość wysłana
siteinfo-key-error = Komunikat o błędzie
sitemenus-title = Menu witryny
sitemenus-help = Listy linków używane ponownie przez menu stron (nagłówek, menu mobilne, stopka): link zmieniony tutaj zmienia się wszędzie
sitemenus-none = Brak menu: zaznacz listę linków na stronie, a potem „Utwórz menu z tymi linkami” w Stylach.
sitemenus-name = Nazwa menu
sitemenus-remove = Usuń to menu (dopiero gdy nie jest już używane)
sitemenus-new = Nazwa nowego menu
sitemenus-add = Nowe menu
sitemenus-list = Menu witryny
sitemenus-links-of = Linki menu
sitemenus-edit = Edytuj linki (Ustawienia › Nawigacja)
sitemenus-written = Linki wpisane ręcznie: { $count }
sitemenus-create = Utwórz menu z tymi linkami
sitemenus-link = Powiąż z menu
sitemenus-link-help = Wpisane linki, które są w tym menu, zostają zastąpione listą menu; pozostałe zostają wpisane
sitemenus-item = Link menu „{ $menu }”
sitemenus-item-help = Każdy link menu powtarza ten element ze swoim tekstem i adresem
sitemenus-item-label = Tekst każdego linku
sitemenus-item-url = Adres każdego linku

# Part roles (carousel, video player, form)
role-slider = W karuzeli
role-slider-help = Rola elementu w zawierającej go karuzeli lub pokazie slajdów
role-slider-track = Ścieżka (przewija się)
role-slider-prev = Przycisk „Poprzedni”
role-slider-next = Przycisk „Następny”
role-slider-count = Licznik (1 / 5)
role-slider-dots = Kropki (tworzone przez witrynę)
role-player = W odtwarzaczu wideo
role-player-help = Rola elementu w zawierającym go odtwarzaczu wideo
role-player-cover = Okładka (uruchamia wideo)
role-cart = Koszyk
role-cart-help = Koszyk witryny: elementy dodawane przyciskami „Dodaj do koszyka”, przechowywane w przeglądarce odwiedzającego, zamówienie wysyłane formularzem koszyka
role-cart-currency = Waluta
role-cart-currency-help = Waluta cen i sumy (kod trzyliterowy)
role-cart-item = Wiersz pozycji (szablon)
role-cart-name = Nazwa pozycji
role-cart-qty = Ilość
role-cart-price = Cena wiersza
role-cart-plus = Przycisk „+”
role-cart-minus = Przycisk „−”
role-cart-remove = Przycisk „Usuń”
role-cart-total = Suma
role-cart-empty = Komunikat „Pusty koszyk”
role-cart-field = Pole wysyłane z zamówieniem
role-cart-add = Dodaj do koszyka
role-cart-add-help = Przycisk dodający pozycję do koszyka: produkt i cena (wpisane lub powiązane z polem wpisu)
role-cart-product = Produkt
role-cart-product-help = Nazwa pozycji w koszyku i w zamówieniu
role-cart-price-value = Cena
role-cart-price-help = Cena jednej sztuki (liczba, bez waluty)
role-cart-count = Liczba pozycji w koszyku
role-cart-count-help = Pokazuje liczbę pozycji w koszyku (pasek nawigacji)
role-remove = Usuń tę rolę
role-form = Formularz witryny
role-form-help = Formularz wysyłany e-mailem przez Twój hosting (Ustawienia › Formularze)
role-form-send = Wysyłany przez witrynę
role-form-send-help = Pola są wysyłane e-mailem na adres ustawiony w Ustawienia › Formularze
role-form-name = Nazwa formularza
role-form-name-help = Nazwa używana w temacie e-maila („kontakt”)
role-form-settings = Ustawienia formularza
role-form-status = Komunikat po wysłaniu

## Valeurs reliées des attributs
bound-value = Wartość
bound-before = Tekst przed
bound-after = Tekst po
bound-only-if = Tylko gdy wartość jest wypełniona
bound-only-if-help = Tekst przed, wartość i tekst po są zapisywane tylko wtedy, gdy wartość jest wypełniona; teksty stałe są zapisywane zawsze („Odtwórz wideo: Tytuł”, w przeciwnym razie „Odtwórz wideo”).
bound-fixed = Stały tekst przed
bound-tail = Stały tekst po
bound-link = Powiąż z wartością (pole, właściwość, informacje o witrynie)
bound-unlink = Odłącz: zamiast tego wpisany tekst
bound-count = Liczba wpisów › { $collection }
bound-style-prop = Właściwość stylu
bound-style-remove = Usuń właściwość
bound-style-add = Powiązana właściwość
condition-field-page = Strona wpisu (wpis ma stronę)
block-bound-format = Format
block-bound-format-help = Sposób zapisu wartości: data lub format witryny (Ustawienia › Filtry).
block-bound-join = Wartości „{ $field }”
block-bound-join-sep = Separator
block-bound-fallback-text = Tekst, gdy puste
block-bound-fallback-text-help = Zapisywany, gdy pole (i pole zastępcze) jest puste.
condition-field-first = Pierwszy z listy
condition-field-last = Ostatni z listy
bound-fallback = Gdy puste
bound-fallback-help = Tekst zapisywany, gdy wartość jest pusta.
role-picker = Rola
role-picker-help = Co ten element robi w otaczającym go elemencie interaktywnym lub na stronie (karta, strzałka karuzeli, przycisk filtra, przycisk zgody, przełącznik dźwięku…).
pages-file-row-help = Plik serwera: jego tekst edytuje się w jego ustawieniach
canvas-render-error = Strony nie można już wyświetlić po tej zmianie (poprzednia pozostaje pokazana; Ctrl+Z ją cofa): { $error }
# Plan du site (D-103)
map-search = Znajdź stronę
map-shared = Części wspólne
map-shared-help = Menu witryny, komponenty i szablony, których linki pojawiają się na kilku stronach
map-external = Inne witryny
map-external-help = Linki do innych witryn, pogrupowane według domeny
map-count-pages = { $count ->
    [one] 1 strona
    [few] { $count } strony
    [many] { $count } stron
   *[other] { $count } strony
}
map-count-broken = { $count ->
    [one] 1 adres bez strony
    [few] { $count } adresy bez strony
    [many] { $count } adresów bez strony
   *[other] { $count } adresu bez strony
}
map-count-orphans = { $count ->
    [one] 1 strona bez linku
    [few] { $count } strony bez linku
    [many] { $count } stron bez linku
   *[other] { $count } strony bez linku
}
map-broken-help = Linki prowadzą do adresu witryny, który nie ma strony (na czerwono): utwórz stronę, przekieruj adres lub zmień linki
map-orphan-help = Żaden link nie prowadzi do tej strony: odwiedzający docierają do niej tylko przez jej adres
map-media = Multimedia i media społecznościowe
map-media-help = Wideo (YouTube, Vimeo…), dźwięk i sieci społecznościowe używane przez witrynę, pogrupowane według platformy; ich adresy są sprawdzane
map-kind-media = Wideo i dźwięk
map-kind-social = Sieć społecznościowa
map-urls-count = { $count ->
    [one] { $count } adres
    [few] { $count } adresy
    [many] { $count } adresów
   *[other] { $count } adresu
}
map-urls = Adresy ({ $count })
map-urls-check = Sprawdź
map-urls-check-help = Sprawdź ponownie każdy adres (wyniki są zachowywane przez dobę)
map-urls-checking = Sprawdzanie adresów…
map-urls-review = { $count ->
    [one] { $count } adres do sprawdzenia
    [few] { $count } adresy do sprawdzenia
    [many] { $count } adresów do sprawdzenia
   *[other] { $count } adresu do sprawdzenia
}
map-urls-review-help = Adresy innych witryn, które nie istnieją, nie działają, są prywatne lub zostały przeniesione: pokaż inne witryny i multimedia
map-url-ok = Online
map-url-redirected = Przeniesiony
map-url-missing = Nie znaleziono
map-url-restricted = Prywatny
map-url-blocked = Nie można sprawdzić
map-url-failing = Nie działa
map-url-unreachable = Brak odpowiedzi
map-url-unchecked = Nie sprawdzono
map-url-final = Prowadzi do
map-url-use-final = Użyj tego adresu
map-url-open = Otwórz w przeglądarce
map-url-replace = Zastąp
map-url-replace-help = Zastąp ten adres wszędzie, gdzie jest używany (strony, multimedia, menu, wpisy CMS, informacje o witrynie)
map-url-replace-title = Zastąp adres wszędzie
map-url-new = Nowy adres
map-url-replaced = { $count ->
    [one] Adres zastąpiony w { $count } miejscu: { $url }
   *[other] Adres zastąpiony w { $count } miejscach: { $url }
}
map-url-uses = { $count ->
    [one] Używany w { $count } miejscu
   *[other] Używany w { $count } miejscach
}
map-url-dblclick = Dwuklik: otwórz miejsce, w którym użyto tego adresu (treść CMS lub strona)
map-source-info = Informacje o witrynie › { $name }
map-source-cms = CMS › { $name }
map-via-src = Osadzone multimedia
map-via-field = Pole „{ $field }”
map-via-body = Tekst CMS
map-via-info = Informacje o witrynie
map-arrange = Uporządkuj
map-arrange-help = Ustaw każdy węzeł z powrotem w kolejności adresów, także przesunięte węzły (Ctrl+Z to cofa)
map-show-in-editor = Pokaż w Edytorze
map-retargeted = { $count ->
    [one] Link prowadzi teraz do { $to }
    [few] { $count } linki prowadzą teraz do { $to }
    [many] { $count } linków prowadzi teraz do { $to }
   *[other] { $count } linku prowadzi teraz do { $to }
}
map-linked-menu = Link do { $to } dodany do menu
map-linked-page = Link do { $to } dodany na końcu strony
map-entries = { $count ->
    [0] Brak wpisów
    [one] 1 wpis
    [few] { $count } wpisy
    [many] { $count } wpisów
   *[other] { $count } wpisu
}
map-entries-help = Pokaż lub ukryj wpisy kolekcji na mapie
map-entries-show = Pokaż wpisy
map-entries-hide = Ukryj wpisy
map-draft = Wersja robocza
map-draft-help = Strona robocza: nieopublikowana
map-port-help = Przeciągnij na stronę, aby dodać do niej link
map-menu-links = { $count ->
    [one] 1 link
    [few] { $count } linki
    [many] { $count } linków
   *[other] { $count } linku
}
map-shared-pages = { $count ->
    [0] Na żadnej stronie
    [one] Na 1 stronie
   *[other] Na { $count } stronach
}
map-minimap = Przegląd: kliknij lub przeciągnij, aby przesunąć widok
map-zoom-out = Pomniejsz (−)
map-zoom-in = Powiększ (+)
map-zoom-reset = Rozmiar rzeczywisty (0)
map-fit = Dopasuj całą mapę (F)
map-key-link = Link
map-key-list = Lista wpisów
map-key-menu = Menu
map-key-redirect = Przekierowanie
map-key-broken = Brak strony
map-panel = Zaznaczenie
map-edge = Linki
map-retarget = Prowadzi do
map-retarget-help = Zmień, dokąd prowadzą te linki (linki wpisane, linki menu, przekierowania; lista podąża za swoją kolekcją). Koniec zaznaczonej linii można też przeciągnąć na inną stronę.
map-retarget-choose = Wybierz stronę…
map-links-count = { $count ->
    [one] 1 link
    [few] { $count } linki
    [many] { $count } linków
   *[other] { $count } linku
}
map-links-in = Linki do niej ({ $count })
map-links-out = Linki z niej ({ $count })
map-none = Brak
map-via-list = Lista wpisów
map-via-menu = Menu witryny
map-via-menu-item = Link menu
map-via-redirect = Przekierowanie
map-via-arg = Wartość komponentu
map-redirected-help = Adres wpisany w linku jest przekierowywany (301) na tę stronę
map-kind-page = Strona
map-kind-collection = Kolekcja
map-kind-entry = Wpis
map-kind-layout = Szablon
map-kind-component = Komponent
map-kind-menu = Menu witryny
map-kind-missing = Adres bez strony
map-kind-external = Inna witryna
map-page-name = Nazwa
map-page-path = Adres
map-page-path-help = Zmiana adresu opublikowanej strony dodaje przekierowanie ze starego adresu
map-page-path-taken = Nieprawidłowy adres lub już używany przez inną stronę
map-open = Otwórz w Edytorze
map-new-child = Nowa strona wewnątrz
map-open-entry = Edytuj wpis
map-open-template = Edytuj szablon
map-open-template-help = Otwiera stronę pierwszego wpisu w Edytorze: zmiany dotyczą każdego wpisu
map-open-cms = Otwórz w CMS
map-edit-menus = Edytuj menu
map-menu-help = Przeciągnij jego punkt na stronę, aby dodać do niej link z tego menu.
map-shared-note = Jego linki są na każdej stronie, która je pokazuje.
map-missing-help = Linki prowadzą do tego adresu, ale żadna strona go nie ma.
map-missing-redirected = Ten stary adres przekierowuje na inną stronę.
map-create-page = Utwórz tę stronę
map-redirect = Przekieruj do
map-redirect-help = Przekierowanie trwałe (301) z tego adresu na stronę
map-open-site = Otwórz witrynę
view-map = Plan witryny

# Paramètres (D-104)
settings-general-lead = Nazwa, adres publiczny i język witryny, kanały RSS, struktura i środowiska.
settings-seo-lead = Mapa witryny, robots.txt, obraz do udostępniania i organizacja: to, co czytają wyszukiwarki i sieci społecznościowe.
settings-publish-lead = Gdzie witryna jest publikowana (hosting SFTP): sprawdzana i wysyłana stąd.
settings-app-general-lead = Język, motyw i kolor Revel oraz folder nowych witryn.
settings-accounts-lead = Twoje zdjęcie, pokazywane w prawym górnym rogu, Twoje imię i nazwisko, zapisywane w wersjach, które zapisujesz, oraz konta używane przez Revel.
siteinfo-remove-used = Nadal używane: odłącz je tam, gdzie jest używane, zanim je usuniesz

# Profil (D-106)
profile-title = Profil
profile-picture = Zdjęcie
profile-picture-help = Pokazywane w prawym górnym rogu. Bez własnego zdjęcia: zdjęcie połączonego konta GitHub.
profile-picture-change = Zmień zdjęcie
profile-picture-remove = Usuń
profile-picture-github = Użyj zdjęcia z GitHub
settings-profile = Profil i konta
error-profile-picture = Nie można odczytać tego obrazu: wybierz plik PNG, JPEG, WebP, GIF lub AVIF.
