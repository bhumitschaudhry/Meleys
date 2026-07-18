// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  docsSidebar: [
    {
      type: 'doc',
      id: 'intro',
    },
    {
      type: 'doc',
      id: 'setup',
    },
    {
      type: 'doc',
      id: 'configuration',
    },
    {
      type: 'doc',
      id: 'api',
    },
    {
      type: 'doc',
      id: 'mcp',
    },
    {
      type: 'doc',
      id: 'architecture',
    },
    {
      type: 'category',
      label: 'Superpowers',
      items: [
        {
          type: 'category',
          label: 'Implementation Plans',
          items: ['superpowers/plans/2026-07-18-multi-engine-architecture'],
        },
        {
          type: 'category',
          label: 'Design Specs',
          items: ['superpowers/specs/2026-07-18-multi-engine-architecture-design'],
        },
      ],
    },
  ],
};

export default sidebars;
