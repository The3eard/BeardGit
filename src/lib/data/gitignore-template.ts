/**
 * Multipurpose `.gitignore` template embedded in the app.
 *
 * Used by the InitRepoDialog when the user picks "initialize this folder"
 * and the folder doesn't already have a `.gitignore`. Covers OS metadata,
 * common IDEs/editors, and the most-used language ecosystems.
 *
 * If you find yourself adding a new section: keep entries alphabetical
 * within their section and stick to the comment-banner style below.
 */
export const MULTI_PURPOSE_GITIGNORE = `# ─── OS ──────────────────────────────────────────────
# macOS
.DS_Store
.AppleDouble
.LSOverride
._*
.Spotlight-V100
.Trashes
# Linux
*~
.fuse_hidden*
.directory
.Trash-*
# Windows
Thumbs.db
ehthumbs.db
Desktop.ini
$RECYCLE.BIN/

# ─── Editors / IDEs ─────────────────────────────────
# Vim / Neovim
*.swp
*.swo
Session.vim
.netrwhist
# Emacs
\\#*\\#
.\\#*
# Sublime Text
*.sublime-workspace
# VS Code
.vscode/
*.code-workspace
# JetBrains (IntelliJ, WebStorm, GoLand, etc.)
.idea/
*.iml
*.ipr
*.iws
.shelf/
# Zed
.zed/
# Visual Studio
.vs/
*.user
*.suo
*.userprefs
# Xcode
xcuserdata/
*.xcworkspace/xcuserdata/

# ─── Languages / Ecosystems ─────────────────────────
# Node
node_modules/
.npm
.yarn/*
!.yarn/releases
!.yarn/patches
!.yarn/plugins
!.yarn/sdks
!.yarn/versions
.pnpm-store/
# Rust
target/
# Python
__pycache__/
*.py[cod]
.venv/
venv/
*.egg-info/
.pytest_cache/
.mypy_cache/
.ruff_cache/
# Java
*.class
*.jar
*.war
.gradle/
build/
# .NET
bin/
obj/
*.dll
*.exe
*.pdb
# Go
*.test
*.out
vendor/
# Ruby
.bundle/
vendor/bundle
*.gem
# PHP
composer.lock
# C / C++
*.o
*.obj
*.so
*.a
*.dylib
# Swift
.build/
Packages/
Package.resolved

# ─── Misc ───────────────────────────────────────────
.env
.env.*
*.log
logs/
tmp/
dist/
out/
coverage/
.cache/
`;
