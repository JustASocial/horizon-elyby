# Ely.by sign-in setup

Register an Ely.by OAuth application with the `Website` application type and use this exact redirect URI:

`https://justasocial.github.io/horizon-elyby/ely-callback/`

The launcher requests `account_info`, `minecraft_server_session`, and `offline_access`. Its default client ID is `horizon-ely1`; `HORIZON_ELY_CLIENT_ID` can override that value for local development.

The token endpoint requires the application client secret. Horizon Revo reads it from `HORIZON_ELY_CLIENT_SECRET` at runtime, or from `%APPDATA%\Horizon Revo\ely-by-client-secret.txt`. Create the `Horizon Revo` folder under the path shown by `%APPDATA%`, then save only the secret in `ely-by-client-secret.txt` and restart the launcher. The file stays in your Windows user profile and is not part of the installer. Do not add the secret to source files, build arguments, GitHub Pages, or a public release.

GitHub Pages only hosts the redirect page. A public desktop release cannot safely keep a confidential OAuth client secret. To distribute Ely.by sign-in to other users, place code exchange and refresh behind a trusted HTTPS service and keep the secret on that service. The local file setup is suitable only for a controlled local build.

After Ely.by returns the authorization code, the static page forwards it to the launcher's temporary loopback listener. The launcher verifies the OAuth state before exchanging the code and storing the resulting tokens in the operating system keychain.
