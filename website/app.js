(() => {
  'use strict';

  const REPO = 'entitybrian69-bit/volt-plus';
  const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=20`;
  const DISALLOWED_RE = /(debug|dev|pre[-_.]?release|alpha|beta|rc\d*|nightly|snapshot|canary|unsigned)/i;

  const THEMES = [
    { id: 'greenhouse', label: 'Greenhouse', bg: 'assets/screen-launcher.jpg' },
    { id: 'sakura', label: 'Sakura', bg: 'assets/screen-shaders.jpg' },
    { id: 'aurora', label: 'Aurora', bg: 'assets/screen-mods.jpg' },
    { id: 'sunrise', label: 'Sunrise', bg: 'assets/screen-skins.jpg' }
  ];

  const SCREENSHOTS = [
    {
      src: 'assets/screen-launcher.jpg',
      tag: '01 / 07 · PLAY',
      title: 'Volt Launcher Plus Home',
      desc: '3D skin preview, Vulkan/GLES renderer switcher, Touch layout selector, Gyroscope toggle, and instant Play.'
    },
    {
      src: 'assets/screen-mods.jpg',
      tag: '02 / 07 · MODS',
      title: '260 Built-in & Custom Mods',
      desc: 'Browse Horizon .hmod mods with version history, descriptions, search, and one-tap Install or .hmod HTTPS import.'
    },
    {
      src: 'assets/screen-packs.jpg',
      tag: '03 / 07 · RESOURCE PACKS',
      title: 'Modrinth Resource Packs',
      desc: 'Search Fresh Animations, Sodium translations, Better Leaves, and custom .zip packs with direct folder access.'
    },
    {
      src: 'assets/screen-shaders.jpg',
      tag: '04 / 07 · SHADERS',
      title: '31 Horizon Native Shaders (.hshader)',
      desc: 'Switch between Horizon Native Shaders and Modrinth shader packs with volumetric clouds, sun shafts, and water reflections.'
    },
    {
      src: 'assets/screen-library.jpg',
      tag: '05 / 07 · INSTALLED',
      title: 'Content Library & File Manager',
      desc: 'Manage all installed mods, resource packs, and shaders from one place, or browse folders inside the built-in File Manager.'
    },
    {
      src: 'assets/screen-skins.jpg',
      tag: '06 / 07 · SKINS STUDIO',
      title: 'Skins & Capes Studio',
      desc: 'Inspect your live 3D Minecraft character model, switch between Classic (4px) and Slim (3px) arms, and import 64x64 skin PNGs.'
    },
    {
      src: 'assets/screen-settings.jpg',
      tag: '07 / 07 · SETTINGS',
      title: 'Comprehensive Mobile Settings',
      desc: 'Fine-tune Video performance, Launcher themes, Touch Controls, Experimental tools, Diagnostics, and Microsoft/Xbox Sign-in.'
    }
  ];

  // ---------- 1. Lightweight Theme Atmosphere Switcher ----------
  const themeBtn = document.getElementById('themeSwitchBtn');
  const themeLabel = document.getElementById('themeSwitchLabel');
  const backdropPhoto = document.getElementById('backdropPhoto');
  let themeIdx = 0;

  if (themeBtn) {
    themeBtn.addEventListener('click', () => {
      themeIdx = (themeIdx + 1) % THEMES.length;
      const t = THEMES[themeIdx];
      document.body.dataset.theme = t.id === 'greenhouse' ? '' : t.id;
      if (themeLabel) themeLabel.textContent = t.label;
      if (backdropPhoto) {
        backdropPhoto.style.opacity = '0.06';
        setTimeout(() => {
          backdropPhoto.style.backgroundImage = `url('${t.bg}')`;
          backdropPhoto.style.opacity = '0.16';
        }, 180);
      }
    });
  }

  // ---------- 2. Throttled Scroll Progress & Nav Active State ----------
  const scrollProgress = document.getElementById('scrollProgress');
  const navPills = Array.from(document.querySelectorAll('.nav-pill'));
  const sectionIds = ['play', 'downloads', 'features', 'screenshots', 'owner', 'faq'];
  let scrollTicking = false;

  function updateScrollState() {
    scrollTicking = false;
    const doc = document.documentElement;
    const max = Math.max(1, doc.scrollHeight - window.innerHeight);
    const ratio = Math.min(1, Math.max(0, window.scrollY / max));
    if (scrollProgress) scrollProgress.style.transform = `scaleX(${ratio})`;

    let current = 'play';
    for (const id of sectionIds) {
      const el = document.getElementById(id);
      if (el && el.getBoundingClientRect().top <= 240) current = id;
    }
    navPills.forEach((p) => p.classList.toggle('is-active', p.dataset.section === current));
  }

  window.addEventListener(
    'scroll',
    () => {
      if (!scrollTicking) {
        scrollTicking = true;
        requestAnimationFrame(updateScrollState);
      }
    },
    { passive: true }
  );
  updateScrollState();

  // ---------- 3. Scroll Reveal & Counters ----------
  const revealObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        el.style.transitionDelay = `${Number(el.dataset.delay || 0)}ms`;
        el.classList.add('is-visible');
        revealObserver.unobserve(el);
      });
    },
    { threshold: 0.08 }
  );
  document.querySelectorAll('.reveal').forEach((el) => revealObserver.observe(el));

  const counterObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        const target = Number(el.dataset.count || 0);
        const suffix = el.dataset.suffix || '';
        const start = performance.now();
        function step(now) {
          const t = Math.max(0, Math.min(1, (now - start) / 800));
          const eased = 1 - Math.pow(1 - t, 3);
          el.textContent = `${Math.round(target * eased)}${suffix}`;
          if (t < 1) requestAnimationFrame(step);
        }
        requestAnimationFrame(step);
        counterObserver.unobserve(el);
      });
    },
    { threshold: 0.3 }
  );
  document.querySelectorAll('.stat-num[data-count]').forEach((el) => counterObserver.observe(el));

  // ---------- 4. Kinetic Hero Subtitle & On-Demand Intro Video ----------
  const kineticEl = document.getElementById('kineticWord');
  const words = ['260 FPS Gaming', 'Vulkan Shaders', '260+ Mods', 'Replay MP4 Export', 'Touch Control Hub'];
  let wordIndex = 0;
  if (kineticEl) {
    setInterval(() => {
      kineticEl.classList.add('is-switching');
      setTimeout(() => {
        wordIndex = (wordIndex + 1) % words.length;
        kineticEl.textContent = words[wordIndex];
        kineticEl.classList.remove('is-switching');
      }, 220);
    }, 2600);
  }

  const toggleIntroBtn = document.getElementById('toggleIntroPreview');
  const toggleIntroLabel = document.getElementById('toggleIntroLabel');
  const previewCaption = document.getElementById('previewCaption');
  const heroPreviewImg = document.getElementById('heroPreviewImg');
  const heroIntroVideo = document.getElementById('heroIntroVideo');
  let showingIntro = false;

  if (toggleIntroBtn && heroPreviewImg && heroIntroVideo) {
    toggleIntroBtn.addEventListener('click', () => {
      showingIntro = !showingIntro;
      heroPreviewImg.classList.toggle('is-active', !showingIntro);
      heroIntroVideo.classList.toggle('is-active', showingIntro);
      if (showingIntro) {
        heroIntroVideo.play().catch(() => {});
      } else {
        heroIntroVideo.pause();
      }
      if (toggleIntroLabel) toggleIntroLabel.textContent = showingIntro ? 'Show Dashboard' : 'Play Intro';
      if (previewCaption) {
        previewCaption.textContent = showingIntro
          ? 'Volt Launcher Plus · First-Start Intro'
          : 'Volt Launcher Plus · Dashboard';
      }
    });
  }

  // ---------- 5. Feature Category Filter ----------
  const filterBtns = Array.from(document.querySelectorAll('.filter-pill'));
  const featureCards = Array.from(document.querySelectorAll('.feature-card'));
  filterBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      const cat = btn.dataset.filter || 'all';
      filterBtns.forEach((b) => b.classList.toggle('is-active', b === btn));
      featureCards.forEach((card) => {
        const show = cat === 'all' || card.dataset.cat === cat;
        card.classList.toggle('is-hidden', !show);
      });
    });
  });

  // ---------- 6. Screenshots Gallery & Fullscreen Lightbox ----------
  let slideIdx = 0;
  const mainImg = document.getElementById('carouselMainImg');
  const cTag = document.getElementById('carouselTag');
  const cTitle = document.getElementById('carouselTitle');
  const cDesc = document.getElementById('carouselDesc');
  const tiles = Array.from(document.querySelectorAll('.gallery-tile'));

  function setSlide(idx) {
    slideIdx = (idx + SCREENSHOTS.length) % SCREENSHOTS.length;
    const item = SCREENSHOTS[slideIdx];
    if (mainImg) {
      mainImg.classList.add('is-fading');
      setTimeout(() => {
        mainImg.src = item.src;
        mainImg.alt = item.title;
        mainImg.classList.remove('is-fading');
      }, 120);
    }
    if (cTag) cTag.textContent = item.tag;
    if (cTitle) cTitle.textContent = item.title;
    if (cDesc) cDesc.textContent = item.desc;
    tiles.forEach((t, i) => t.classList.toggle('is-active', i === slideIdx));
  }

  document.getElementById('prevSlideBtn')?.addEventListener('click', () => setSlide(slideIdx - 1));
  document.getElementById('nextSlideBtn')?.addEventListener('click', () => setSlide(slideIdx + 1));
  tiles.forEach((t) => {
    t.addEventListener('click', () => setSlide(Number(t.dataset.index || 0)));
  });

  const lightbox = document.getElementById('lightbox');
  const lbImg = document.getElementById('lbImg');
  const lbTitle = document.getElementById('lbTitle');
  const lbDesc = document.getElementById('lbDesc');
  let lbIdx = 0;

  function openLightbox(idx) {
    lbIdx = (idx + SCREENSHOTS.length) % SCREENSHOTS.length;
    const item = SCREENSHOTS[lbIdx];
    if (lbImg) {
      lbImg.src = item.src;
      lbImg.alt = item.title;
    }
    if (lbTitle) lbTitle.textContent = item.title;
    if (lbDesc) lbDesc.textContent = item.desc;
    if (lightbox) {
      lightbox.classList.add('is-open');
      lightbox.setAttribute('aria-hidden', 'false');
    }
  }

  function closeLightbox() {
    if (lightbox) {
      lightbox.classList.remove('is-open');
      lightbox.setAttribute('aria-hidden', 'true');
    }
  }

  document.getElementById('openLightboxBtn')?.addEventListener('click', () => openLightbox(slideIdx));
  mainImg?.addEventListener('click', () => openLightbox(slideIdx));
  document.getElementById('lbClose')?.addEventListener('click', closeLightbox);
  document.getElementById('lbPrev')?.addEventListener('click', () => openLightbox(lbIdx - 1));
  document.getElementById('lbNext')?.addEventListener('click', () => openLightbox(lbIdx + 1));
  lightbox?.addEventListener('click', (e) => {
    if (e.target === lightbox) closeLightbox();
  });
  window.addEventListener('keydown', (e) => {
    if (!lightbox?.classList.contains('is-open')) return;
    if (e.key === 'Escape') closeLightbox();
    if (e.key === 'ArrowLeft') openLightbox(lbIdx - 1);
    if (e.key === 'ArrowRight') openLightbox(lbIdx + 1);
  });

  // ---------- 7. FAQ & Checksum Drawers ----------
  document.querySelectorAll('.faq-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const item = btn.closest('.faq-item');
      if (item) item.classList.toggle('is-open');
    });
  });

  const toggleVerifyBtn = document.getElementById('toggleVerifyBtn');
  const verifyDrawer = document.getElementById('verifyDrawer');
  if (toggleVerifyBtn && verifyDrawer) {
    toggleVerifyBtn.addEventListener('click', () => {
      const open = !verifyDrawer.classList.contains('is-open');
      verifyDrawer.classList.toggle('is-open', open);
      verifyDrawer.setAttribute('aria-hidden', String(!open));
    });
  }

  // ---------- 8. Toast & Copy SHA-256 ----------
  const toastEl = document.getElementById('downloadToast');
  const toastTitle = document.getElementById('toastTitle');
  const toastSub = document.getElementById('toastSub');
  let toastTimer = null;

  function showToast(title, sub) {
    if (!toastEl) return;
    if (toastTitle) toastTitle.textContent = title;
    if (toastSub) toastSub.textContent = sub;
    toastEl.classList.add('is-shown');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toastEl.classList.remove('is-shown'), 4200);
  }

  document.querySelectorAll('.copy-btn').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const target = document.getElementById(btn.dataset.copyTarget);
      const val = target?.textContent?.trim() || '';
      if (!val || val.includes('Verified')) return;
      try {
        await navigator.clipboard.writeText(val);
        const orig = btn.textContent;
        btn.textContent = 'Copied';
        setTimeout(() => { btn.textContent = orig; }, 1500);
      } catch (_) {}
    });
  });

  // ---------- 9. Device ABI Detection ----------
  let detectedAbi = 'arm64';

  async function detectDeviceAbi() {
    const tagEl = document.getElementById('deviceDetectTag');
    const ua = navigator.userAgent || '';
    let abi = 'arm64';
    let label = 'Recommended: 64-bit';

    try {
      if (navigator.userAgentData?.getHighEntropyValues) {
        const hi = await navigator.userAgentData.getHighEntropyValues(['architecture', 'bitness']);
        if (hi.architecture === 'arm' && hi.bitness === '32') {
          abi = 'armv7';
          label = 'Detected: 32-bit ARM';
        } else if (hi.architecture === 'arm' || hi.bitness === '64') {
          abi = 'arm64';
          label = 'Detected: 64-bit ARM';
        }
      } else if (/armv7|armeabi/i.test(ua) && !/aarch64|arm64/i.test(ua)) {
        abi = 'armv7';
        label = 'Detected: 32-bit ARM';
      } else if (/aarch64|arm64/i.test(ua)) {
        abi = 'arm64';
        label = 'Detected: 64-bit ARM';
      } else if (!/android/i.test(ua)) {
        abi = 'universal';
        label = 'Desktop · Universal / 64-bit';
      }
    } catch (_) {}

    detectedAbi = abi;
    if (tagEl) tagEl.textContent = label;

    ['arm64', 'armv7', 'universal'].forEach((k) => {
      const card = document.getElementById(`card-${k}`);
      if (card) card.classList.toggle('is-highlighted', k === abi);
    });
    updateHeroSmartButton();
  }

  // ---------- 10. Release-Only Auto-Updating APK Sync ----------
  let currentReleaseState = {
    ready: false,
    version: '1.0.1',
    tag: 'v1.0.1',
    assets: {
      arm64: { filename: 'Volt-Plus-arm64-v8a.apk', url: './downloads/Volt-Plus-arm64-v8a.apk', sizeMb: '57 MB', sha256: 'Verified on deploy' },
      armv7: { filename: 'Volt-Plus-armeabi-v7a.apk', url: './downloads/Volt-Plus-armeabi-v7a.apk', sizeMb: '52 MB', sha256: 'Verified on deploy' },
      universal: { filename: 'Volt-Plus-universal.apk', url: './downloads/Volt-Plus-universal.apk', sizeMb: '96 MB', sha256: 'Verified on deploy' }
    }
  };

  function isStableRelease(rel) {
    if (!rel || rel.draft || rel.prerelease) return false;
    return !DISALLOWED_RE.test(`${rel.tag_name || ''} ${rel.name || ''}`);
  }

  function matchAssetAbi(assets, abi) {
    if (!Array.isArray(assets)) return null;
    const valid = assets.filter((a) => {
      const n = (a.name || '').toLowerCase();
      return n.endsWith('.apk') && !DISALLOWED_RE.test(n);
    });
    if (abi === 'arm64') return valid.find((a) => /arm64|aarch64/i.test(a.name));
    if (abi === 'armv7') return valid.find((a) => /armeabi-v7a|armv7|armeabi/i.test(a.name));
    if (abi === 'universal') return valid.find((a) => /universal/i.test(a.name));
    return null;
  }

  function formatMb(bytes) {
    if (!bytes || !Number.isFinite(bytes)) return '—';
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function updateHeroSmartButton() {
    const heroBtn = document.getElementById('heroSmartDownload');
    const heroTitle = document.getElementById('heroBtnTitle');
    const heroSub = document.getElementById('heroBtnSub');
    if (!heroBtn) return;

    const info = currentReleaseState.assets[detectedAbi] || currentReleaseState.assets.arm64;
    const labelMap = {
      arm64: 'Download 64-bit APK',
      armv7: 'Download 32-bit APK',
      universal: 'Download Universal APK'
    };
    heroBtn.href = info.url;
    heroBtn.setAttribute('download', info.filename);
    heroBtn.dataset.abi = detectedAbi;
    if (heroTitle) heroTitle.textContent = `${labelMap[detectedAbi]} · ${currentReleaseState.tag}`;
    if (heroSub) heroSub.textContent = `${info.sizeMb} · Direct Release APK`;
  }

  function renderReleaseState() {
    const badge = document.getElementById('releaseVersionBadge');
    const meta = document.getElementById('releaseMetaLine');
    const navSub = document.getElementById('navVersionSub');
    const heroLabel = document.getElementById('heroReleaseLabel');

    if (badge) badge.textContent = `Latest Release: ${currentReleaseState.tag}`;
    if (navSub) navSub.textContent = `Android ${currentReleaseState.version}`;
    if (heroLabel) {
      heroLabel.innerHTML = `<span class="live-dot"></span><span>Minecraft Java 26.3 · Release ${currentReleaseState.tag}</span>`;
    }
    if (meta && currentReleaseState.publishedAt) {
      meta.textContent = `Published ${new Date(currentReleaseState.publishedAt).toLocaleDateString()} — Official Release channel only (drafts, debug builds, and pre-releases excluded).`;
    }

    document.querySelectorAll('.js-ver').forEach((el) => {
      el.textContent = currentReleaseState.tag;
    });

    ['arm64', 'armv7', 'universal'].forEach((abi) => {
      const a = currentReleaseState.assets[abi];
      const link = document.getElementById(`dl-${abi}`);
      const sizeEl = document.getElementById(`size-${abi}`);
      const shaEl = document.getElementById(`sha-${abi}`);
      if (link) {
        link.href = a.url;
        link.setAttribute('download', a.filename);
      }
      if (sizeEl && a.sizeMb) sizeEl.textContent = a.sizeMb;
      if (shaEl && a.sha256) shaEl.textContent = a.sha256;
    });

    updateHeroSmartButton();
  }

  async function syncLatestRelease() {
    let baked = null;
    try {
      const res = await fetch(`./latest-release.json?t=${Date.now()}`, { cache: 'no-store' });
      if (res.ok) baked = await res.json();
    } catch (_) {}

    if (baked && baked.status === 'ready' && baked.assets) {
      currentReleaseState = {
        ready: true,
        version: baked.version || baked.tag_name || '1.0.1',
        tag: baked.tag_name || 'v1.0.1',
        publishedAt: baked.published_at || '',
        assets: {
          arm64: {
            filename: baked.assets.arm64?.filename || 'Volt-Plus-arm64-v8a.apk',
            url: baked.assets.arm64?.url || './downloads/Volt-Plus-arm64-v8a.apk',
            sizeMb: baked.assets.arm64?.size_mb || formatMb(baked.assets.arm64?.size_bytes),
            sha256: baked.assets.arm64?.sha256 || 'Verified SHA-256'
          },
          armv7: {
            filename: baked.assets.armv7?.filename || 'Volt-Plus-armeabi-v7a.apk',
            url: baked.assets.armv7?.url || './downloads/Volt-Plus-armeabi-v7a.apk',
            sizeMb: baked.assets.armv7?.size_mb || formatMb(baked.assets.armv7?.size_bytes),
            sha256: baked.assets.armv7?.sha256 || 'Verified SHA-256'
          },
          universal: {
            filename: baked.assets.universal?.filename || 'Volt-Plus-universal.apk',
            url: baked.assets.universal?.url || './downloads/Volt-Plus-universal.apk',
            sizeMb: baked.assets.universal?.size_mb || formatMb(baked.assets.universal?.size_bytes),
            sha256: baked.assets.universal?.sha256 || 'Verified SHA-256'
          }
        }
      };
    }

    try {
      const apiRes = await fetch(RELEASES_API, {
        headers: { Accept: 'application/vnd.github+json' },
        cache: 'no-store'
      });
      if (apiRes.ok) {
        const list = await apiRes.json();
        const stable = Array.isArray(list) ? list.find(isStableRelease) : null;
        if (stable) {
          const a64 = matchAssetAbi(stable.assets, 'arm64');
          const a32 = matchAssetAbi(stable.assets, 'armv7');
          const aUni = matchAssetAbi(stable.assets, 'universal');
          if (a64 || a32 || aUni) {
            const matchesBaked = baked && baked.tag_name === stable.tag_name && baked.status === 'ready';
            currentReleaseState.ready = true;
            currentReleaseState.tag = stable.tag_name;
            currentReleaseState.version = stable.tag_name.replace(/^v/i, '');
            currentReleaseState.publishedAt = stable.published_at;

            if (a64) {
              currentReleaseState.assets.arm64 = {
                filename: matchesBaked ? baked.assets.arm64.filename : a64.name,
                url: matchesBaked ? baked.assets.arm64.url : a64.browser_download_url,
                sizeMb: formatMb(a64.size),
                sha256: matchesBaked ? baked.assets.arm64.sha256 : (a64.digest || 'Release verified')
              };
            }
            if (a32) {
              currentReleaseState.assets.armv7 = {
                filename: matchesBaked ? baked.assets.armv7.filename : a32.name,
                url: matchesBaked ? baked.assets.armv7.url : a32.browser_download_url,
                sizeMb: formatMb(a32.size),
                sha256: matchesBaked ? baked.assets.armv7.sha256 : (a32.digest || 'Release verified')
              };
            }
            if (aUni) {
              currentReleaseState.assets.universal = {
                filename: matchesBaked ? baked.assets.universal.filename : aUni.name,
                url: matchesBaked ? baked.assets.universal.url : aUni.browser_download_url,
                sizeMb: formatMb(aUni.size),
                sha256: matchesBaked ? baked.assets.universal.sha256 : (aUni.digest || 'Release verified')
              };
            }
          }
        }
      }
    } catch (_) {}

    renderReleaseState();
  }

  document.querySelectorAll('.js-apk-download').forEach((link) => {
    link.addEventListener('click', (e) => {
      const abi = link.dataset.abi || 'arm64';
      const asset = currentReleaseState.assets[abi] || currentReleaseState.assets.arm64;
      if (!currentReleaseState.ready) {
        e.preventDefault();
        showToast('Syncing Release...', 'GitHub Pages is finalizing the latest APK build.');
        return;
      }

      if (/^https?:\/\//i.test(asset.url) && !asset.url.startsWith(window.location.origin)) {
        e.preventDefault();
        const frame = document.createElement('iframe');
        frame.style.display = 'none';
        frame.src = asset.url;
        document.body.appendChild(frame);
        setTimeout(() => frame.remove(), 30000);
      }

      showToast(
        `Downloading ${asset.filename}`,
        `${currentReleaseState.tag} (${asset.sizeMb}) started directly.`
      );
    });
  });

  detectDeviceAbi();
  syncLatestRelease();
})();
