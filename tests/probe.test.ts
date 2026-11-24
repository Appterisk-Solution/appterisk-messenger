import { afterEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { boundsFor, messageFor } from '../src/services/probe';
import App from '../src/CompatibilityProbe.vue';
import { flushPromises } from '@vue/test-utils';

afterEach(() => { vi.unstubAllGlobals(); });
describe('compatibility shell', () => {
  it('reports browser-only native IPC failure and leaves retry available', async () => {
    vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
    const wrapper = mount(App);
    const button = wrapper.findAll('button').find(item => item.text() === 'Test local isolation')!;
    await button.trigger('click');
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('Run npm run tauri dev');
    expect(button.attributes('disabled')).toBeUndefined();
    expect(wrapper.findAll('.native-slot')).toHaveLength(3);
    wrapper.unmount();
  });
  it('preserves logical pixel bounds, including fractional DPI values', () => {
    const element = document.createElement('div');
    element.getBoundingClientRect = () => ({ x: 72.5, y: 90, width: 400.25, height: 600 }) as DOMRect;
    expect(boundsFor([element])).toEqual([{ x: 72.5, y: 90, width: 400.25, height: 600 }]);
  });
  it('handles structured and unexpected failures', () => {
    expect(messageFor({ code: 'FORBIDDEN', message: 'Denied' })).toBe('Denied');
    expect(messageFor('Offline')).toBe('Offline');
    expect(messageFor(null)).toContain('Native operation failed');
  });
});
