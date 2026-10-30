import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'CLI ツールで学ぶ Rust',
  tagline: '1 本のコマンドを作りながら、言語の判断を辿る',

  // GitHub Pages の既定の置き場所。リポジトリ名が baseUrl になる。
  url: 'https://karak.github.io',
  baseUrl: '/rust-learn/',
  organizationName: 'karak',
  projectName: 'rust-learn',

  future: {
    v4: true,
  },

  // **リンク切れでビルドを落とす。** 本は docs/adr/ を参照するが、
  // サイトの外にある文書は GitHub の URL で指す決まりにした
  // (ADR-0013 論点 7)。相対リンクを書くとここで落ちる。
  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'ja',
    locales: ['ja'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          // **本そのものがサイトである。** /docs/ を挟まない。
          routeBasePath: '/',
          editUrl: 'https://github.com/karak/rust-learn/tree/main/book/',
        },
        // 本に連載は無い。
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'CLI ツールで学ぶ Rust',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'bookSidebar',
          position: 'left',
          label: '目次',
        },
        {
          href: 'https://github.com/karak/rust-learn',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: '題材',
          items: [
            {
              label: 'tally のソース',
              href: 'https://github.com/karak/rust-learn/tree/main/crates/tally',
            },
            {
              label: '出力の契約',
              href: 'https://github.com/karak/rust-learn/blob/main/crates/tally/docs/output-format.md',
            },
          ],
        },
        {
          title: '設計判断の正本',
          items: [
            {
              label: 'ADR 一覧',
              href: 'https://github.com/karak/rust-learn/tree/main/docs/adr',
            },
          ],
        },
      ],
      copyright: `MIT License. 本文とコードは <a href="https://github.com/karak/rust-learn">rust-learn</a> にあります。`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      // Prism の既定に Rust と TOML は入っていない。
      additionalLanguages: ['rust', 'toml', 'bash'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
