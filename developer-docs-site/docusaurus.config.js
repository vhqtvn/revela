// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

const codeInjector = require("./src/remark/code-injector");

/** @type {import("@docusaurus/types").Config} */
const config = {
  title: "Aptos Labs",
  tagline: "Developer Documentation",
  url: "https://aptos.dev",
  baseUrl: "/",
  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",
  favicon: "img/favicon.ico",
  organizationName: "aptos-labs", // Usually your GitHub org/user name.
  projectName: "aptos-core", // Usually your repo name.

  presets: [
    [
      "@docusaurus/preset-classic",
      /** @type {import("@docusaurus/preset-classic").Options} */
      ({
        docs: {
          routeBasePath: "/",
          sidebarPath: require.resolve("./sidebars.js"),
          sidebarCollapsible: false,
          editUrl: "https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/",
          remarkPlugins: [codeInjector],
        },
        blog: false,
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
        gtag: {
          trackingID: "G-HVB7QFB9PQ",
        },
      }),
    ],
    [
      "redocusaurus",
      {
        // Plugin Options for loading OpenAPI files
        specs: [
          {
            spec: "../api/doc/v0/openapi.yaml",
            route: "/rest-api/",
          },
        ],
        // Theme Options for modifying how redoc renders them
        theme: {
          // Change with your site colors
          primaryColor: "#1890ff",
        },
      },
    ],
  ],

  themeConfig:
    /** @type {import("@docusaurus/preset-classic").ThemeConfig} */
    ({
      colorMode: {
        defaultMode: "dark",
      },
      docs: {
        sidebar: {
          autoCollapseCategories: true,
          hideable: true,
        },
      },
      navbar: {
        title: "| Developer Network",
        logo: {
          alt: "Aptos Labs Logo",
          src: "img/aptos_word.svg",
          srcDark: "img/aptos_word_dark.svg",
        },
        items: [
          {
            href: "https://github.com/aptos-labs/aptos-core/",
            label: "GitHub",
            position: "right",
          },
          {
            type: "dropdown",
            label: "Move",
            position: "left",
            items: [
              {
                label: "Move on Aptos",
                type: "doc",
                docId: "guides/move-guides/move-on-aptos",
              },
              {
                label: "Your First Move Module",
                type: "doc",
                docId: "tutorials/first-move-module",
              },
            ],
          },
          {
            type: "dropdown",
            label: "Applications",
            position: "left",
            items: [
              {
                type: "doc",
                label: "Your First Transaction",
                docId: "tutorials/first-transaction",
              },
              {
                type: "doc",
                label: "Your First DApp",
                docId: "tutorials/first-dapp",
              },
              {
                type: "doc",
                label: "Your First Coin",
                docId: "tutorials/first-coin",
              },
              {
                type: "doc",
                label: "Your First NFT",
                docId: "tutorials/your-first-nft",
              },
            ],
          },
          {
            type: "dropdown",
            label: "Nodes",
            to: "nodes/nodes-index",
            position: "left",
            items: [
              {
                label: "Aptos Blockchain Deployments",
                type: "doc",
                docId: "nodes/aptos-deployments",
              },
              {
                label: "Validators",
                type: "doc",
                docId: "nodes/validator-node/index",
              },
              {
                label: "FullNodes",
                type: "doc",
                docId: "nodes/full-node/index",
              },
              {
                label: "Local Testnet",
                type: "doc",
                docId: "nodes/run-a-local-testnet",
              },
              {
                label: "Node Health Checker",
                type: "doc",
                docId: "nodes/node-health-checker",
              },
              {
                label: "Staking",
                type: "doc",
                docId: "nodes/staking",
              },
            ],
          },
          {
            position: "left",
            href: "https://fullnode.devnet.aptoslabs.com/spec.html#/",
            label: "REST API",
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: null,
            items: [
              {
                html: `
                  <a class="social-link" href="https://aptoslabs.com" target="_blank" rel="noopener noreferrer" title="Git">
                     <img class="logo" src="/img/aptos_word_dark.svg" alt="Aptos Logo" />
                  </a>
                `,
              },
            ],
          },
          {
            title: null,
            items: [
              {
                html: `
                <p class="emails">
                  If you have any questions, please contact us at </br>
                  <a href="mailto:info@aptoslabs.com" target="_blank" rel="noreferrer noopener">
                    info@aptoslabs.com
                  </a> or
                  <a href="mailto:press@aptoslabs.com" target="_blank" rel="noreferrer noopener">
                    press@aptoslabs.com
                  </a>
                </p>
              `,
              },
            ],
          },
          {
            title: null,
            items: [
              {
                html: `
                  <p class="right">
                    <nav class="social-links">
                        <a class="social-link" href="https://github.com/aptoslabs" target="_blank" rel="noopener noreferrer" title="Git">
                         <img class="icon" src="/img/socials/git.svg" alt="Git Icon" />
                        </a>
                        <a class="social-link" href="https://discord.gg/aptoslabs" target="_blank" rel="noopener noreferrer" title="Discord">
                          <img class="icon" src="/img/socials/discord.svg" alt="Discord Icon" />
                        </a>
                        <a class="social-link" href="https://twitter.com/aptoslabs/" target="_blank" rel="noopener noreferrer" title="Twitter">
                          <img class="icon" src="/img/socials/twitter.svg" alt="Twitter Icon" />
                        </a>
                        <a class="social-link" href="https://aptoslabs.medium.com/" target="_blank" rel="noopener noreferrer" title="Medium">
                          <img class="icon" src="/img/socials/medium.svg" alt="Medium Icon" />
                        </a>
                        <a class="social-link" href="https://www.linkedin.com/company/aptoslabs/" target="_blank" rel="noopener noreferrer" title="LinkedIn">
                          <img class="icon" src="/img/socials/linkedin.svg" alt="LinkedIn Icon" />
                        </a>
                    </nav>
                  </p>
              `,
              },
            ],
          },
        ],
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ["rust"],
      },
      algolia: {
        appId: "HM7UY0NMLG",
        apiKey: "63c5819714b74e64977337e61a1e3ae6",
        indexName: "aptos",
        contextualSearch: true,
        debug: false,
      },
    }),
  plugins: [
    [
      "@docusaurus/plugin-client-redirects",
      {
        redirects: [
          {
            to: "/nodes/full-node/fullnode-for-devnet",
            from: "/tutorials/run-a-fullnode",
          },
          {
            to: "/nodes/aptos-deployments",
            from: "/tutorials/local-testnet-devnet-and-incentivized-testnet",
          },
          {
            to: "/nodes/full-node/run-a-fullnode-on-gcp",
            from: "/tutorials/run-a-fullnode-on-gcp",
          },
          {
            to: "/nodes/ait/node-requirements",
            from: "/tutorials/validator-node/intro",
          },
          {
            to: "/nodes/validator-node/validators",
            from: [
              "/tutorials/validator-node/run-validator-node-using-gcp",
              "/tutorials/validator-node/run-validator-node-using-aws",
              "/tutorials/validator-node/run-validator-node-using-azure",
              "/tutorials/validator-node/run-validator-node-using-docker",
              "/tutorials/validator-node/run-validator-node-using-source",
            ],
          },
          {
            to: "/nodes/ait/connect-to-testnet",
            from: "/tutorials/validator-node/connect-to-testnet",
          },
          {
            to: "/nodes/ait/node-liveness-criteria",
            from: "/reference/node-liveness-criteria",
          },
          {
            to: "/concepts/aptos-concepts",
            from: [
              "/basics/basics-txns-states",
              "/basics/basics-accounts",
              "/basics/basics-events",
              "/basics/basics-gas-txn-fee",
              "/basics/basics-merkle-proof",
              "/basics/basics-fullnodes",
              "/basics/basics-validator-nodes",
              "/basics/basics-node-networks-sync",
            ],
          },
        ],
      },
    ],
  ],
};

module.exports = config;
