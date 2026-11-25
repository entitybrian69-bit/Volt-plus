(() => {
  'use strict';

  const REPO = 'entitybrian69-bit/volt-plus';
  const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=20`;
  const DISALLOWED_RE = /(debug|dev|pre[-_.]?release|alpha|beta|rc\d*|nightly|snapshot|canary|unsigned)/i;

  const SCREENSHOTS = [
    {
      src: 'assets/screen-launcher.jpg',
      tag: '01 / 07 · PLAY DASHBOARD',
      title: 'Volt Launcher Plus Home',
      desc: '3D skin preview, quick Vulkan/GLES renderer switcher, Touch layout selector, Gyroscope toggle, and instant Play Minecraft 26.3.'
    },
    {
      src: 'assets/screen-mods.jpg',
      tag: '02 / 07 · MODS BROWSER',
      title: '260+ Built-in & Custom Mods',
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
      tag: '04 / 07 · SHADER PACKS',
      title: '31+ Horizon Native Shaders (.hshader)',
      desc: 'Switch between Horizon Native Shaders and Modrinth shader packs with volumetric clouds, sun shafts, and water reflections.'
    },
    {
      src: 'assets/screen-library.jpg',
      tag: '05 / 07 · CONTENT LIBRARY',
      title: 'Content Library & File Manager',
      desc: 'Manage all installed mods, resource packs, and shaders from one place, or browse folders inside the built-in File Manager.'
    },
    {
      src: 'assets/screen-skins.jpg',
      tag: '06 / 07 · SKINS & CAPES STUDIO',
      title: 'Skins & Capes Studio',
      desc: 'Inspect your live 3D Minecraft character model, switch between Classic (4px) and Slim (3px) arms, and import 64x64 skin PNGs.'
    },
    {
      src: 'assets/screen-settings.jpg',
      tag: '07 / 07 · SETTINGS HUB',
      title: 'Comprehensive Mobile Settings',
      desc: 'Fine-tune Video performance, Launcher themes, Touch Controls, Experimental tools, Diagnostics, and Microsoft/Xbox Sign-in.'
    }
  ];

  // ---------- 1. Scroll Progress, Navbar & Back-to-top ----------
  const scrollProgress = document.getElementById('scrollProgress');
  const navbar = document.getElementById('navbar');
  const scrollTopBtn = document.getElementById('scrollTopBtn');
  const navLinks = Array.from(document.querySelectorAll('.nav-link'));

  function onScroll() {
    const doc = document.documentElement;
    const max = Math.max(1, doc.scrollHeight - window.innerHeight);
    const ratio = Math.min(1, Math.max(0, window.scrollY / max));
    if (scrollProgress) scrollProgress.style.transform = `scaleX(${ratio})`;
    if (navbar) navbar.classList.toggle('is-scrolled', window.scrollY > 24);
    if (scrollTopBtn) scrollTopBtn.classList.toggle('is-visible', window.scrollY > 420);

    const sections = ['downloads', 'features', 'screenshots', 'owner', 'faq'];
    let current = '';
    for (const id of sections) {
      const el = document.getElementById(id);
      if (el && el.getBoundingClientRect().top <= 180) {
        current = id;
      }
    }
    navLinks.forEach((link) => {
      link.classList.toggle('is-active', link.getAttribute('href') === `#${current}`);
    });
  }

  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();

  if (scrollTopBtn) {
    scrollTopBtn.addEventListener('click', () => {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    });
  }

  // Mobile Nav Drawer
  const navToggle = document.getElementById('navToggle');
  const mobileDrawer = document.getElementById('mobileDrawer');
  if (navToggle && mobileDrawer) {
    navToggle.addEventListener('click', () => {
      const open = !mobileDrawer.classList.contains('is-open');
      mobileDrawer.classList.toggle('is-open', open);
      navToggle.classList.toggle('is-open', open);
      navToggle.setAttribute('aria-expanded', String(open));
    });
    mobileDrawer.querySelectorAll('a').forEach((a) => {
      a.addEventListener('click', () => {
        mobileDrawer.classList.remove('is-open');
        navToggle.classList.remove('is-open');
        navToggle.setAttribute('aria-expanded', 'false');
      });
    });
  }

  // ---------- 2. Interactive Particle Canvas & Cursor Spotlight ----------
  const canvas = document.getElementById('particleCanvas');
  const spotlight = document.getElementById('cursorSpotlight');
  const pointer = { x: -9999, y: -9999, active: false };

  window.addEventListener('pointermove', (e) => {
    pointer.x = e.clientX;
    pointer.y = e.clientY;
    pointer.active = true;
    if (spotlight) {
      spotlight.style.left = `${e.clientX}px`;
      spotlight.style.top = `${e.clientY}px`;
      spotlight.style.opacity = '1';
    }
  }, { passive: true });

  if (canvas) {
    const ctx = canvas.getContext('2d');
    let width = 0;
    let height = 0;
    let particles = [];

    function resizeCanvas() {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = window.innerWidth;
      height = window.innerHeight;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const count = Math.min(72, Math.max(28, Math.floor((width * height) / 22000)));
      particles = Array.from({ length: count }, () => ({
        x: Math.random() * width,
        y: Math.random() * height,
        vx: (Math.random() - 0.5) * 0.45,
        vy: (Math.random() - 0.5) * 0.45,
        r: Math.random() * 2 + 1,
        hue: Math.random() < 0.5 ? '#7c5cff' : Math.random() < 0.7 ? '#00e5ff' : '#3ddc84'
      }));
    }

    function tickParticles() {
      ctx.clearRect(0, 0, width, height);
      for (let i = 0; i < particles.length; i++) {
        const p = particles[i];
        p.x += p.vx;
        p.y += p.vy;

        if (p.x < 0) p.x = width;
        if (p.x > width) p.x = 0;
        if (p.y < 0) p.y = height;
        if (p.y > height) p.y = 0;

        if (pointer.active) {
          const dx = p.x - pointer.x;
          const dy = p.y - pointer.y;
          const dist = Math.hypot(dx, dy);
          if (dist > 0 && dist < 130) {
            const push = (130 - dist) / 130;
            p.x += (dx / dist) * push * 1.6;
            p.y += (dy / dist) * push * 1.6;
          }
        }

        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = p.hue;
        ctx.globalAlpha = 0.62;
        ctx.fill();

        for (let j = i + 1; j < particles.length; j++) {
          const q = particles[j];
          const dist = Math.hypot(p.x - q.x, p.y - q.y);
          if (dist < 115) {
            ctx.beginPath();
            ctx.moveTo(p.x, p.y);
            ctx.lineTo(q.x, q.y);
            ctx.strokeStyle = 'rgba(130, 150, 255, ' + ((1 - dist / 115) * 0.16).toFixed(3) + ')';
            ctx.lineWidth = 1;
            ctx.stroke();
          }
        }
      }
      ctx.globalAlpha = 1;
      requestAnimationFrame(tickParticles);
    }

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    requestAnimationFrame(tickParticles);
  }

  // ---------- 3. Scroll Reveal & Animated Counters ----------
  const revealObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        const delay = Number(el.dataset.delay || 0);
        el.style.transitionDelay = `${delay}ms`;
        el.classList.add('is-visible');
        revealObserver.unobserve(el);
      });
    },
    { threshold: 0.12 }
  );

  document.querySelectorAll('.reveal').forEach((el) => revealObserver.observe(el));

  const counterObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        const target = Number(el.dataset.count || 0);
        const suffix = el.dataset.suffix || '';
        const duration = 1150;
        const start = performance.now();
        function step(now) {
          const t = Math.min(1, (now - start) / duration);
          const eased = 1 - Math.pow(1 - t, 3);
          el.textContent = `${Math.round(target * eased)}${suffix}`;
          if (t < 1) requestAnimationFrame(step);
        }
        requestAnimationFrame(step);
        counterObserver.unobserve(el);
      });
    },
    { threshold: 0.4 }
  );

  document.querySelectorAll('.stat-num[data-count]').forEach((el) => counterObserver.observe(el));

  // ---------- 4. 3D Tilt Cards & Magnetic Buttons ----------
  if (window.matchMedia('(pointer: fine)').matches) {
    document.querySelectorAll('.tilt-card').forEach((card) => {
      card.addEventListener('pointermove', (e) => {
        const rect = card.getBoundingClientRect();
        const px = (e.clientX - rect.left) / rect.width - 0.5;
        const py = (e.clientY - rect.top) / rect.height - 0.5;
        card.style.transform = `perspective(900px) rotateX(${(-py * 5.5).toFixed(2)}deg) rotateY(${(px * 6.5).toFixed(2)}deg) translateY(-3px)`;
      });
      card.addEventListener('pointerleave', () => {
        card.style.transform = '';
      });
    });

    document.querySelectorAll('.magnetic').forEach((btn) => {
      btn.addEventListener('pointermove', (e) => {
        const rect = btn.getBoundingClientRect();
        const dx = (e.clientX - (rect.left + rect.width / 2)) * 0.18;
        const dy = (e.clientY - (rect.top + rect.height / 2)) * 0.18;
        btn.style.transform = `translate(${dx.toFixed(1)}px, ${dy.toFixed(1)}px)`;
      });
      btn.addEventListener('pointerleave', () => {
        btn.style.transform = '';
      });
    });
  }

  // ---------- 5. Kinetic Hero Words & Intro GIF Toggle ----------
  const kineticEl = document.getElementById('kineticWord');
  const words = ['Android', '260 FPS', 'Vulkan', 'Shaders', 'Java 26.3'];
  let wordIndex = 0;
  if (kineticEl) {
    setInterval(() => {
      kineticEl.classList.add('is-switching');
      setTimeout(() => {
        wordIndex = (wordIndex + 1) % words.length;
        kineticEl.textContent = words[wordIndex];
        kineticEl.classList.remove('is-switching');
      }, 280);
    }, 2400);
  }

  const toggleIntroBtn = document.getElementById('toggleIntroPreview');
  const toggleIntroLabel = document.getElementById('toggleIntroLabel');
  const heroPreviewImg = document.getElementById('heroPreviewImg');
  const heroIntroGif = document.getElementById('heroIntroGif');
  let showingIntro = false;
  if (toggleIntroBtn && heroPreviewImg && heroIntroGif) {
    toggleIntroBtn.addEventListener('click', () => {
      showingIntro = !showingIntro;
      heroPreviewImg.classList.toggle('is-visible', !showingIntro);
      heroIntroGif.classList.toggle('is-visible', showingIntro);
      if (toggleIntroLabel) {
        toggleIntroLabel.textContent = showingIntro ? 'Show Launcher Dashboard' : 'Play Intro Animation';
      }
    });
  }

  // ---------- 6. Feature Tabs ----------
  const tabs = Array.from(document.querySelectorAll('.ftab'));
  const panels = Array.from(document.querySelectorAll('.ftab-panel'));
  tabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      const target = tab.dataset.tab;
      tabs.forEach((t) => {
        const active = t === tab;
        t.classList.toggle('is-active', active);
        t.setAttribute('aria-selected', String(active));
      });
      panels.forEach((p) => {
        p.classList.toggle('is-active', p.dataset.panel === target);
      });
    });
  });

  // ---------- 7. Screenshots Carousel & Lightbox ----------
  let slideIdx = 0;
  const mainImg = document.getElementById('carouselMainImg');
  const cTag = document.getElementById('carouselTag');
  const cTitle = document.getElementById('carouselTitle');
  const cDesc = document.getElementById('carouselDesc');
  const thumbs = Array.from(document.querySelectorAll('.thumb'));

  function setSlide(idx) {
    slideIdx = (idx + SCREENSHOTS.length) % SCREENSHOTS.length;
    const item = SCREENSHOTS[slideIdx];
    if (mainImg) {
      mainImg.classList.add('is-fading');
      setTimeout(() => {
        mainImg.src = item.src;
        mainImg.alt = item.title;
        mainImg.classList.remove('is-fading');
      }, 150);
    }
    if (cTag) cTag.textContent = item.tag;
    if (cTitle) cTitle.textContent = item.title;
    if (cDesc) cDesc.textContent = item.desc;
    thumbs.forEach((t, i) => t.classList.toggle('is-active', i === slideIdx));
  }

  document.getElementById('prevSlideBtn')?.addEventListener('click', () => setSlide(slideIdx - 1));
  document.getElementById('nextSlideBtn')?.addEventListener('click', () => setSlide(slideIdx + 1));
  thumbs.forEach((t) => t.addEventListener('click', () => setSlide(Number(t.dataset.index || 0))));

  let autoSlideTimer = setInterval(() => setSlide(slideIdx + 1), 5200);
  const carouselEl = document.querySelector('.carousel-shell');
  carouselEl?.addEventListener('pointerenter', () => clearInterval(autoSlideTimer));
  carouselEl?.addEventListener('pointerleave', () => {
    clearInterval(autoSlideTimer);
    autoSlideTimer = setInterval(() => setSlide(slideIdx + 1), 5200);
  });

  // Lightbox
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
  document.querySelectorAll('.shot-card').forEach((card) => {
    card.addEventListener('click', () => openLightbox(Number(card.dataset.index || 0)));
  });
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

  // ---------- 8. FAQ Accordion & Release Notes Toggle ----------
  document.querySelectorAll('.faq-q').forEach((btn) => {
    btn.addEventListener('click', () => {
      const item = btn.closest('.faq-item');
      if (item) item.classList.toggle('is-open');
    });
  });

  const releaseNotesBtn = document.getElementById('toggleReleaseNotesBtn');
  const releaseNotesPanel = document.getElementById('releaseNotesPanel');
  if (releaseNotesBtn && releaseNotesPanel) {
    releaseNotesBtn.addEventListener('click', () => {
      const open = !releaseNotesPanel.classList.contains('is-open');
      releaseNotesPanel.classList.toggle('is-open', open);
      releaseNotesPanel.setAttribute('aria-hidden', String(!open));
    });
  }

  // ---------- 9. Toast & Copy SHA-256 ----------
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

  document.querySelectorAll('.copy-sha-btn').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const target = document.getElementById(btn.dataset.copyTarget);
      const val = target?.textContent?.trim() || '';
      if (!val || val.includes('verifying')) return;
      try {
        await navigator.clipboard.writeText(val);
        const orig = btn.textContent;
        btn.textContent = 'Copied ✓';
        setTimeout(() => { btn.textContent = orig; }, 1600);
      } catch (_) {}
    });
  });

  // ---------- 10. Device Architecture Detection ----------
  let detectedAbi = 'arm64';

  async function detectDeviceAbi() {
    const tagEl = document.getElementById('deviceDetectTag');
    const ua = navigator.userAgent || '';
    let abi = 'arm64';
    let label = 'Recommended: 64-bit (arm64-v8a)';

    try {
      if (navigator.userAgentData?.getHighEntropyValues) {
        const hi = await navigator.userAgentData.getHighEntropyValues(['architecture', 'bitness', 'platform']);
        if (hi.architecture === 'arm' && hi.bitness === '32') {
          abi = 'armv7';
          label = 'Detected 32-bit ARM Device';
        } else if (hi.architecture === 'arm' || hi.bitness === '64') {
          abi = 'arm64';
          label = 'Detected 64-bit ARM Device';
        }
      } else if (/armv7|armeabi/i.test(ua) && !/aarch64|arm64/i.test(ua)) {
        abi = 'armv7';
        label = 'Detected 32-bit ARM Device';
      } else if (/aarch64|arm64/i.test(ua)) {
        abi = 'arm64';
        label = 'Detected 64-bit ARM Device';
      } else if (!/android/i.test(ua)) {
        abi = 'universal';
        label = 'Desktop Browser · Universal / 64-bit Recommended';
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

  // ---------- 11. Auto-Updating Release-Only APK Resolver ----------
  let currentReleaseState = {
    ready: false,
    version: '1.0.1',
    tag: 'v1.0.1',
    notes: 'Official signed Volt Plus Release.',
    assets: {
      arm64: { filename: 'Volt-Plus-arm64-v8a.apk', url: './downloads/Volt-Plus-arm64-v8a.apk', sizeMb: '57 MB', sha256: 'Verified on deploy' },
      armv7: { filename: 'Volt-Plus-armeabi-v7a.apk', url: './downloads/Volt-Plus-armeabi-v7a.apk', sizeMb: '52 MB', sha256: 'Verified on deploy' },
      universal: { filename: 'Volt-Plus-universal.apk', url: './downloads/Volt-Plus-universal.apk', sizeMb: '96 MB', sha256: 'Verified on deploy' }
    }
  };

  function isStableRelease(rel) {
    if (!rel || rel.draft || rel.prerelease) return false;
    const text = `${rel.tag_name || ''} ${rel.name || ''}`;
    return !DISALLOWED_RE.test(text);
  }

  function matchAssetAbi(assets, abi) {
    if (!Array.isArray(assets)) return null;
    const valid = assets.filter((a) => {
      const n = (a.name || '').toLowerCase();
      return n.endsWith('.apk') && !DISALLOWED_RE.test(n);
    });
    if (abi === 'arm64') {
      return valid.find((a) => /arm64|aarch64/i.test(a.name));
    }
    if (abi === 'armv7') {
      return valid.find((a) => /armeabi-v7a|armv7|armeabi/i.test(a.name));
    }
    if (abi === 'universal') {
      return valid.find((a) => /universal/i.test(a.name));
    }
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
      arm64: '64-bit APK',
      armv7: '32-bit APK',
      universal: 'Universal APK'
    };
    heroBtn.href = info.url;
    heroBtn.setAttribute('download', info.filename);
    heroBtn.dataset.abi = detectedAbi;
    if (heroTitle) {
      heroTitle.textContent = `Download ${labelMap[detectedAbi]} (${currentReleaseState.tag})`;
    }
    if (heroSub) {
      heroSub.textContent = `${info.sizeMb} · Direct Release APK · No GitHub Redirect`;
    }
  }

  function renderReleaseState() {
    const badge = document.getElementById('releaseVersionBadge');
    const meta = document.getElementById('releaseMetaLine');
    const navSub = document.getElementById('navVersionSub');
    const notesHeading = document.getElementById('releaseNotesHeading');
    const notesBody = document.getElementById('releaseNotesBody');
    const heroLabel = document.getElementById('heroReleaseLabel');

    if (badge) badge.textContent = `Latest Release: ${currentReleaseState.tag}`;
    if (navSub) navSub.textContent = `${currentReleaseState.tag} · Release`;
    if (heroLabel) heroLabel.textContent = `Official Release ${currentReleaseState.tag} · No Debug · No Pre-release`;
    if (meta) {
      meta.textContent = currentReleaseState.publishedAt
        ? `Published ${new Date(currentReleaseState.publishedAt).toLocaleDateString()} · Signed Release builds (64-bit, 32-bit & Universal)`
        : 'Signed Release builds synced directly on this domain (64-bit, 32-bit & Universal)';
    }
    if (notesHeading) notesHeading.textContent = `Release Notes — ${currentReleaseState.tag}`;
    if (notesBody) notesBody.textContent = currentReleaseState.notes || 'Official signed Release build.';

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
    const refreshBtn = document.getElementById('refreshReleaseBtn');
    refreshBtn?.classList.add('is-spinning');

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
        notes: baked.notes || 'Official Volt Plus signed Release.',
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

    // Also poll GitHub Releases API (strictly stable Releases only — never draft, prerelease, or debug)
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
            currentReleaseState.notes = stable.body || currentReleaseState.notes;

            // Keep same-origin ./downloads/*.apk whenever Pages already synced this release so tapping NEVER opens GitHub.
            // If a brand-new Release was just published seconds ago before Pages redeploy finishes, fall back to direct API binary asset URL.
            if (a64) {
              currentReleaseState.assets.arm64 = {
                filename: matchesBaked ? baked.assets.arm64.filename : a64.name,
                url: matchesBaked ? baked.assets.arm64.url : a64.browser_download_url,
                sizeMb: formatMb(a64.size),
                sha256: matchesBaked ? baked.assets.arm64.sha256 : (a64.digest || 'GitHub Release verified')
              };
            }
            if (a32) {
              currentReleaseState.assets.armv7 = {
                filename: matchesBaked ? baked.assets.armv7.filename : a32.name,
                url: matchesBaked ? baked.assets.armv7.url : a32.browser_download_url,
                sizeMb: formatMb(a32.size),
                sha256: matchesBaked ? baked.assets.armv7.sha256 : (a32.digest || 'GitHub Release verified')
              };
            }
            if (aUni) {
              currentReleaseState.assets.universal = {
                filename: matchesBaked ? baked.assets.universal.filename : aUni.name,
                url: matchesBaked ? baked.assets.universal.url : aUni.browser_download_url,
                sizeMb: formatMb(aUni.size),
                sha256: matchesBaked ? baked.assets.universal.sha256 : (aUni.digest || 'GitHub Release verified')
              };
            }
          }
        }
      }
    } catch (_) {}

    renderReleaseState();
    setTimeout(() => refreshBtn?.classList.remove('is-spinning'), 450);
  }

  document.getElementById('refreshReleaseBtn')?.addEventListener('click', () => {
    syncLatestRelease().then(() => {
      showToast('Release Check Complete', `Synced to latest stable Release ${currentReleaseState.tag}.`);
    });
  });

  // Direct download handler: starts download cleanly in-place without navigating away to GitHub
  document.querySelectorAll('.js-apk-download').forEach((link) => {
    link.addEventListener('click', (e) => {
      const abi = link.dataset.abi || 'arm64';
      const asset = currentReleaseState.assets[abi] || currentReleaseState.assets.arm64;
      if (!currentReleaseState.ready) {
        e.preventDefault();
        showToast('Release Sync Pending', 'The GitHub Pages release workflow is still packaging the APKs. Try again in a minute.');
        return;
      }

      // If URL is cross-origin fallback (rare ~60s window right after new release before Pages finishes syncing),
      // trigger download via hidden iframe so the browser downloads the APK without leaving this page.
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
