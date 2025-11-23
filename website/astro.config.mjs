// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import react from '@astrojs/react';

import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  site: 'https://fabriziosalmi.github.io',
  base: '/secbeat',
  integrations: [starlight({
      title: 'SecBeat',
      description: 'DDoS mitigation at kernel speed. eBPF + WASM + Rust.',
      logo: {
        src: './src/assets/logo.svg',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/yourusername/secbeat'
        }
      ],
      customCss: [
        './src/styles/global.css',
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', slug: 'index' },
            { label: 'Quick Start', slug: 'quickstart' },
            { label: 'Installation', slug: 'installation' },
          ],
        },
        {
          label: 'Core Architecture',
          items: [
            { label: 'Overview', slug: 'core/overview' },
            { label: 'Observability', slug: 'core/observability' },
            { label: 'SYN Flood Mitigation', slug: 'core/syn-flood' },
          ],
        },
        {
          label: 'Kernel Layer (eBPF)',
          items: [
            { label: 'XDP Programs', slug: 'kernel/xdp' },
            { label: 'Performance', slug: 'kernel/performance' },
          ],
        },
        {
          label: 'Intelligence Layer (WASM)',
          items: [
            { label: 'WASM Runtime', slug: 'intelligence/wasm-runtime' },
            { label: 'Dynamic Rules', slug: 'intelligence/dynamic-rules' },
            { label: 'Hot Reload', slug: 'intelligence/hot-reload' },
          ],
        },
        {
          label: 'Enterprise Features',
          items: [
            { label: 'Distributed State (CRDTs)', slug: 'enterprise/distributed-state' },
            { label: 'Dashboard', slug: 'enterprise/dashboard' },
            { label: 'Multi-Region', slug: 'enterprise/multi-region' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Configuration', slug: 'reference/config' },
            { label: 'CLI', slug: 'reference/cli' },
            { label: 'API', slug: 'reference/api' },
          ],
        },
      ],
      }), react()],

  vite: {
    plugins: [tailwindcss()],
  },
});