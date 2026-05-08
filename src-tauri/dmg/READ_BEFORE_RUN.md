# READ BEFORE RUN — BeardGit

BeardGit is currently distributed without an Apple Developer ID, so macOS will
block the app on first launch with one of these warnings:

- "BeardGit is damaged and can't be opened"
- "BeardGit can't be opened because the developer cannot be verified"

The app is safe — the warnings exist because the binary is not notarized.

## Unblock it (Terminal — recommended)

1. Drag `BeardGit.app` to `/Applications`.
2. Open Terminal and run:

   ```sh
   xattr -dr com.apple.quarantine /Applications/BeardGit.app
   ```

3. Open BeardGit normally from Launchpad or `/Applications`.

You only need to do this once per install. In-app auto-updates work normally
afterwards.

## Without Terminal

Right-click `BeardGit.app` → **Open** → click **Open** in the confirmation
dialog. Or go to **System Settings → Privacy & Security → Open Anyway**.

---

# LEER ANTES DE EJECUTAR — BeardGit

BeardGit se distribuye sin certificado de Apple, así que macOS bloqueará la app
al abrirla por primera vez con uno de estos avisos:

- "BeardGit está dañada y no se puede abrir"
- "No se puede abrir BeardGit porque no se puede verificar el desarrollador"

La app es segura — el aviso aparece porque el binario no está notarizado.

## Desbloquearla (Terminal — recomendado)

1. Arrastra `BeardGit.app` a `/Aplicaciones`.
2. Abre Terminal y ejecuta:

   ```sh
   xattr -dr com.apple.quarantine /Applications/BeardGit.app
   ```

3. Abre BeardGit normalmente desde Launchpad o `/Aplicaciones`.

Solo hay que hacerlo una vez por instalación. Las actualizaciones automáticas
dentro de la app funcionarán con normalidad después.

## Sin Terminal

Click derecho sobre `BeardGit.app` → **Abrir** → confirmar **Abrir** en el
diálogo. O ir a **Ajustes del Sistema → Privacidad y Seguridad → Abrir
igualmente**.
