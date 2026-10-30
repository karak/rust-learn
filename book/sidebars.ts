import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

/**
 * 目次は自動生成しない。
 *
 * **章の順序は、この本の中心的な判断だからである**
 * (ADR-0013 論点 4。言語機能を軸に、学びの階段の落差が小さくなるよう並べた)。
 * ファイル名の連番に任せると、並べ替えがファイル名の付け替えになり、
 * **なぜその順序なのかを書き残す場所が無くなる。**
 *
 * 書いた章だけをここに足す。**宣言した章のファイルが無ければビルドが落ちる。**
 */
const sidebars: SidebarsConfig = {
  bookSidebar: [
    'intro',
    {
      type: 'category',
      label: 'I 値',
      collapsed: false,
      items: ['part1/ownership'],
    },
  ],
};

export default sidebars;
