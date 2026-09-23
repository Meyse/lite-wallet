import '../../src/app.css';
import { mount } from 'svelte';
import { setLocale } from '$lib/i18n';
import HelpFixture from './HelpFixture.svelte';

const params = new URLSearchParams(location.search);
document.documentElement.classList.toggle('dark', params.get('theme') === 'dark');
setLocale(params.get('locale') ?? 'en');
if (params.has('frame')) {
  mount(HelpFixture, { target: document.getElementById('fixture')! });
} else {
  // The iframe gives the real full-window dialog an exact desktop viewport,
  // independently of the browser panel size. No wallet data or native commands.
  const frame = document.createElement('iframe');
  params.set('frame', '');
  frame.src = `${location.pathname}?${params}`;
  frame.title = 'Wallet help at 920 by 620';
  frame.style.cssText = 'width:920px;height:620px;border:0;display:block;margin:0;';
  document.getElementById('fixture')!.append(frame);
}
