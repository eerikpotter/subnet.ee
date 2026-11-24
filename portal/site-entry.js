// The certified static site serves both domains. Only the active surface starts.
(() => {
  const cloudOrigin = document.querySelector('meta[name="subnet-cloud-origin"]').content;
  const hostname = location.hostname;
  const cloud = hostname === new URL(cloudOrigin).hostname
    || hostname.endsWith('.localhost')
    || /^[a-z0-9-]+\.(?:icp\.net|ic0\.app|icp0\.io)$/.test(hostname);
  document.documentElement.dataset.subnetSite = cloud ? 'cloud' : 'public';
  if (cloud) {
    const style = document.createElement('link');
    style.rel = 'stylesheet';
    style.href = '/style.css';
    document.head.append(style);
  }
  document.addEventListener('DOMContentLoaded', () => {
    const head = document.getElementById('subnet-cloud-head');
    const body = document.getElementById('subnet-cloud-body');
    if (cloud) {
      for (const node of document.head.querySelectorAll('title, meta[name="description"], meta[property^="og:"], link[rel="canonical"], link[rel="alternate"], link[rel="stylesheet"], link[rel="preload"]')) node.remove();
      document.head.append(head.content.cloneNode(true));
      document.body.replaceChildren(body.content.cloneNode(true));
    } else {
      head.remove();
      body.remove();
    }
    document.documentElement.dataset.subnetReady = '';
    // i18n's DOMContentLoaded listener runs before these downloaded scripts.
    for (const src of cloud ? ['/workspace.js', '/cloud.js'] : ['/landing.js', '/agent-workflow.js']) {
      const script = document.createElement('script');
      script.src = src;
      script.async = false;
      if (src === '/cloud.js') script.type = 'module';
      document.head.append(script);
    }
  }, { once: true });
})();
