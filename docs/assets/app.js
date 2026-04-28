/* BeardGit landing — interactions */
(() => {
  const root = document.documentElement;
  const STORAGE_KEY = "beardgit.theme";

  /* ---------- Theme cycle: auto → light → dark ---------- */
  const ICONS = {
    auto:  '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor" stroke="none"/>',
    light: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>',
    dark:  '<path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"/>'
  };
  const LABELS = { auto: "Auto", light: "Light", dark: "Dark" };
  const ORDER = ["auto", "light", "dark"];

  const toggle = document.getElementById("themeToggle");
  const iconEl = document.getElementById("themeIcon");
  const labelEl = document.getElementById("themeLabel");

  const mqlLight = window.matchMedia("(prefers-color-scheme: light)");

  function effectiveTheme() {
    const t = root.getAttribute("data-theme") || "auto";
    if (t === "light" || t === "dark") return t;
    return mqlLight.matches ? "light" : "dark";
  }

  function syncScreenshotVariants() {
    const theme = effectiveTheme();
    // Update <source srcset> first so the picture re-evaluation sees the new
    // candidates when we then update the <img src> below.
    document.querySelectorAll(".shot-src").forEach((src) => {
      const target = theme === "light" ? src.dataset.srcsetLight : src.dataset.srcsetDark;
      if (!target) return;
      if (src.getAttribute("srcset") !== target) src.setAttribute("srcset", target);
    });
    document.querySelectorAll(".shot-img").forEach((img) => {
      const target = theme === "light" ? img.dataset.srcLight : img.dataset.srcDark;
      if (!target) return;
      const abs = new URL(target, location.href).href;
      if (img.src !== abs) img.src = target;
    });
  }

  function applyTheme(mode) {
    root.setAttribute("data-theme", mode);
    if (iconEl) iconEl.innerHTML = ICONS[mode];
    if (labelEl) labelEl.textContent = LABELS[mode];
    if (mode === "auto") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, mode);
    syncScreenshotVariants();
  }

  mqlLight.addEventListener?.("change", () => {
    if ((root.getAttribute("data-theme") || "auto") === "auto") syncScreenshotVariants();
  });

  const saved = localStorage.getItem(STORAGE_KEY);
  const initial = ORDER.includes(saved) ? saved : "auto";
  applyTheme(initial);

  toggle?.addEventListener("click", () => {
    const current = root.getAttribute("data-theme") || "auto";
    const next = ORDER[(ORDER.indexOf(current) + 1) % ORDER.length];
    applyTheme(next);
  });

  /* ---------- Reveal on scroll ---------- */
  root.classList.add("js-reveal");
  const io = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) {
          e.target.classList.add("in");
          io.unobserve(e.target);
        }
      });
    },
    { rootMargin: "0px 0px -8% 0px", threshold: 0.08 }
  );
  document.querySelectorAll(".reveal").forEach((el) => io.observe(el));

  /* ---------- Stagger hero children ---------- */
  document.querySelectorAll(".hero .reveal").forEach((el, i) => {
    el.style.transitionDelay = `${i * 90}ms`;
  });

  /* ---------- Copy-to-clipboard on install cards ---------- */
  document.querySelectorAll(".install-copy").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const span = btn.parentElement.querySelector("[data-cmd]");
      if (!span) return;
      try {
        await navigator.clipboard.writeText(span.dataset.cmd);
        const original = btn.textContent;
        btn.textContent = "Copied";
        btn.classList.add("copied");
        setTimeout(() => {
          btn.textContent = original;
          btn.classList.remove("copied");
        }, 1600);
      } catch {
        btn.textContent = "Err";
      }
    });
  });

  /* ---------- Screenshot placeholders: hide label once image loads ---------- */
  document.querySelectorAll(".screenshot-slot img").forEach((img) => {
    const slot = img.closest(".screenshot-slot");
    if (!slot) return;
    const markLoaded = () => {
      if (img.naturalWidth > 0) slot.classList.add("loaded");
      else slot.classList.add("missing");
    };
    if (img.complete) markLoaded();
    else {
      img.addEventListener("load", markLoaded);
      img.addEventListener("error", () => slot.classList.add("missing"));
    }
  });

  /* ---------- Lightbox: click any screenshot to enlarge ----------
     On phones (≤480px) the lightbox chrome is too small to be usable, so we
     skip it entirely and open the full-size image in a new tab. A companion
     CSS rule (@media max-width: 480px .lightbox { display:none }) guarantees
     the overlay stays hidden even if something else flips the .open class. */
  const lightbox = document.getElementById("lightbox");
  const lightboxImg = document.getElementById("lightboxImg");
  const lightboxCaption = document.getElementById("lightboxCaption");
  const lightboxClose = document.getElementById("lightboxClose");
  const mqlPhone = window.matchMedia("(max-width: 480px)");

  function openLightbox(src, caption, alt) {
    if (!lightbox || !lightboxImg) return;
    lightboxImg.src = src;
    lightboxImg.alt = alt || "";
    if (lightboxCaption) lightboxCaption.textContent = caption || "";
    lightbox.classList.add("open");
    lightbox.setAttribute("aria-hidden", "false");
    document.body.style.overflow = "hidden";
  }
  function closeLightbox() {
    if (!lightbox) return;
    lightbox.classList.remove("open");
    lightbox.setAttribute("aria-hidden", "true");
    document.body.style.overflow = "";
  }

  document.querySelectorAll(".shot-figure").forEach((figure) => {
    const slot = figure.querySelector(".screenshot-slot");
    const img = figure.querySelector(".shot-img");
    const caption = figure.querySelector(".shot-caption");
    if (!slot || !img) return;
    slot.addEventListener("click", () => {
      const src = img.currentSrc || img.src;
      // Phone fallback: open the image in a new tab — the lightbox overlay
      // is too small to be useful on a 375px viewport.
      if (mqlPhone.matches) {
        window.open(src, "_blank", "noopener");
        return;
      }
      openLightbox(src, caption?.textContent || "", img.alt);
    });
  });

  lightbox?.addEventListener("click", (e) => {
    if (e.target === lightbox) closeLightbox();
  });
  lightboxClose?.addEventListener("click", closeLightbox);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && lightbox?.classList.contains("open")) closeLightbox();
  });

  /* ---------- Keep hero stats subtly alive ---------- */
  document.querySelectorAll(".stat-num").forEach((el) => {
    el.addEventListener("mouseenter", () => {
      el.style.transition = "letter-spacing .4s";
      el.style.letterSpacing = "-0.02em";
    });
    el.addEventListener("mouseleave", () => {
      el.style.letterSpacing = "";
    });
  });

  /* ---------- Resolve download links against the latest GitHub release ----------
     Every link with data-dl="macos|linux|windows" is re-pointed at the matching
     release asset (dmg, AppImage, exe). The hero button with data-dl="auto"
     picks the asset that matches the visitor's OS via userAgent. All failures
     silently fall back to /releases/latest. */
  const REPO = "The3eard/BeardGit";
  const OS_PATTERNS = {
    macos:   /(aarch64|apple[-_]?silicon|arm64).*\.dmg$/i,
    linux:   /\.AppImage$/i,
    windows: /(setup|installer).*\.exe$|\.exe$/i,
  };
  const OS_LABELS = {
    macos:   "Download for macOS",
    linux:   "Download for Linux",
    windows: "Download for Windows",
  };

  function detectOS() {
    const ua = (navigator.userAgent || "") + " " + (navigator.platform || "");
    if (/Mac|iPhone|iPad/i.test(ua)) return "macos";
    if (/Win/i.test(ua)) return "windows";
    if (/Linux|X11/i.test(ua)) return "linux";
    return null;
  }

  function pickAsset(assets, re) {
    const match = assets.find((a) => re.test(a.name));
    return match ? match.browser_download_url : null;
  }

  async function fetchLatestRelease() {
    // /releases/latest skips pre-releases and drafts, which 404s while we're
    // still in beta. Fall back to the full /releases list and take the first
    // non-draft entry (GitHub returns them newest-first).
    const stable = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (stable.ok) return stable.json();

    const list = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=10`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!list.ok) return null;
    const releases = await list.json();
    return Array.isArray(releases) ? releases.find((r) => !r.draft) : null;
  }

  function relativeTime(iso) {
    const then = new Date(iso).getTime();
    if (!Number.isFinite(then)) return "";
    const diff = Date.now() - then;
    const day = 86400000;
    if (diff < day) return "today";
    if (diff < day * 2) return "yesterday";
    if (diff < day * 30) return `${Math.round(diff / day)} days ago`;
    if (diff < day * 365) return `${Math.round(diff / (day * 30))} months ago`;
    return `${Math.round(diff / (day * 365))} years ago`;
  }

  function fillReleaseBadge(data) {
    const badge = document.querySelector("[data-release-badge]");
    if (!badge) return;
    const tag = data.tag_name || data.name;
    if (!tag) return;
    const when = data.published_at || data.created_at;
    const rel = when ? ` · released ${relativeTime(when)}` : "";
    badge.textContent = `${tag}${rel}`;
    badge.hidden = false;
  }

  async function wireDownloads() {
    try {
      const data = await fetchLatestRelease();
      if (!data) return;
      fillReleaseBadge(data);
      const assets = Array.isArray(data.assets) ? data.assets : [];
      const urls = {
        macos:   pickAsset(assets, OS_PATTERNS.macos)   || pickAsset(assets, /\.dmg$/i),
        linux:   pickAsset(assets, OS_PATTERNS.linux),
        windows: pickAsset(assets, OS_PATTERNS.windows),
      };

      document.querySelectorAll("[data-dl]").forEach((el) => {
        const key = el.dataset.dl;
        if (key === "auto") {
          const os = detectOS();
          const url = os && urls[os];
          if (url) {
            el.href = url;
            const lbl = el.querySelector("[data-dl-label]");
            if (lbl && OS_LABELS[os]) lbl.textContent = OS_LABELS[os];
          }
          return;
        }
        if (urls[key]) el.href = urls[key];
      });
    } catch {
      /* offline / rate-limited — keep the /releases/latest fallback */
    }
  }

  wireDownloads();
})();
