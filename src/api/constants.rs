/// Librus Synergia REST API base URL
pub const LIBRUS_API_URL: &str = "https://api.librus.pl/2.0";

/// OAuth Token endpoint for Synergia login (username/password & refresh)
pub const LIBRUS_API_TOKEN_URL: &str = "https://api.librus.pl/OAuth/Token";

/// OAuth Token endpoint for JST login (code+pin)
pub const LIBRUS_API_TOKEN_JST_URL: &str = "https://api.librus.pl/OAuth/TokenJST";

/// Basic auth header value for Synergia OAuth (client_id:secret base64)
pub const LIBRUS_API_AUTHORIZATION: &str = "Basic Mjg6ODRmZGQzYTg3YjAzZDNlYTZmZmU3NzdiNThiMzMyYjE=";

/// User-Agent string mimicking the Librus mobile app
pub const LIBRUS_USER_AGENT: &str = "Dalvik/2.1.0 Android LibrusMobileApp";

/// Portal OAuth2 URLs (for future email-based login)
pub const LIBRUS_PORTAL_URL: &str = "https://portal.librus.pl/api";
pub const LIBRUS_TOKEN_URL: &str = "https://portal.librus.pl/oauth2/access_token";
pub const LIBRUS_CLIENT_ID: &str = "VaItV6oRutdo8fnjJwysnTjVlvaswf52ZqmXsJGP";
pub const LIBRUS_REDIRECT_URL: &str = "app://librus";
pub const LIBRUS_AUTHORIZE_URL: &str = "https://portal.librus.pl/konto-librus/redirect/dru";

/// Synergia web URLs
pub const LIBRUS_SYNERGIA_URL: &str = "https://synergia.librus.pl";

/// Messages API URL
pub const LIBRUS_MESSAGES_URL: &str = "https://wiadomosci.librus.pl/module";

/// User-Agent for Synergia web / messages
pub const SYNERGIA_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Gecko/20100101 Firefox/62.0";

/// Portal login action URL
pub const LIBRUS_LOGIN_URL: &str = "https://portal.librus.pl/konto-librus/login/action";

/// X-Requested-With header value
pub const LIBRUS_HEADER: &str = "pl.librus.synergiaDru2";

/// Portal API: list of Synergia accounts
pub const LIBRUS_ACCOUNTS_URL: &str = "/v3/SynergiaAccounts";

/// Portal API: fresh token for a specific Synergia account (append /{login})
pub const LIBRUS_ACCOUNT_URL: &str = "/v3/SynergiaAccounts/fresh";
