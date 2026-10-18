/* Navigation stays usable while the cloud client starts or reconnects. */
(() => {
  const toggle = document.getElementById('workspace-menu');
  const navigation = document.getElementById('workspace-navigation');
  const accountMenu = document.getElementById('account-menu');
  const content = document.getElementById('workspace-content');
  const desktop = window.matchMedia('(min-width: 761px)');
  document.body.classList.add('workspace-enhanced');

  function label() {
    toggle.setAttribute('aria-label', window.SubnetI18n.t(toggle.getAttribute('aria-expanded') === 'true' ? 'Sulge menüü' : 'Ava menüü'));
  }
  function closeNavigation(returnFocus = false) {
    navigation.removeAttribute('data-open');
    toggle.setAttribute('aria-expanded', 'false');
    label();
    if (returnFocus) toggle.focus();
  }
  document.getElementById('workspace-skip').addEventListener('click', event => {
    event.preventDefault();
    closeNavigation();
    content.focus();
  });
  toggle.addEventListener('click', () => {
    if (toggle.getAttribute('aria-expanded') === 'true') closeNavigation();
    else {
      accountMenu.open = false;
      navigation.setAttribute('data-open', '');
      toggle.setAttribute('aria-expanded', 'true');
      label();
    }
  });
  navigation.addEventListener('click', event => {
    if (event.target.closest('a[href^="#"]') && !desktop.matches) {
      closeNavigation();
      content.focus({ preventScroll: true });
    }
  });
  document.addEventListener('click', event => {
    if (!navigation.contains(event.target) && !toggle.contains(event.target)) closeNavigation();
    if (!accountMenu.contains(event.target)) accountMenu.open = false;
  });
  document.addEventListener('keydown', event => {
    if (event.key !== 'Escape') return;
    if (toggle.getAttribute('aria-expanded') === 'true') closeNavigation(true);
    if (accountMenu.open) {
      accountMenu.open = false;
      accountMenu.querySelector('summary').focus();
    }
  });
  document.addEventListener('focusin', event => {
    if (!navigation.contains(event.target) && !toggle.contains(event.target)) closeNavigation();
    if (!accountMenu.contains(event.target)) accountMenu.open = false;
  });
  desktop.addEventListener('change', () => closeNavigation());
  window.addEventListener('subnet:languagechange', label);
})();
