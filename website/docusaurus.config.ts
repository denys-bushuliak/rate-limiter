import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const config: Config = {
	title: "Rate Limiter",
	tagline: "High-performance, lock-free rate limiting in Rust",
	favicon: "img/favicon.ico",

	// Future flags, see https://docusaurus.io/docs/api/docusaurus-config#future
	future: {
		v4: true, // Improve compatibility with the upcoming Docusaurus v4
	},

	// Set the production url of your site here
	url: "https://denys.github.io",
	// Set the /<baseUrl>/ pathname under which your site is served
	// For GitHub pages deployment, it is often "/<projectName>/"
	baseUrl: "/rate-limiter/",

	// GitHub pages deployment config.
	// If you aren't using GitHub pages, you don't need these.
	organizationName: "denys", // Usually your GitHub org/user name.
	projectName: "rate-limiter", // Usually your repo name.

	onBrokenLinks: "throw",

	markdown: {
		mermaid: true,
	},
	themes: ["@docusaurus/theme-mermaid"],

	// Even if you don't use internationalization, you can use this field to set
	// useful metadata like html lang. For example, if your site is Chinese, you
	// may want to replace "en" with "zh-Hans".
	i18n: {
		defaultLocale: "en",
		locales: ["en"],
	},

	presets: [
		[
			"classic",
			{
				docs: {
					sidebarPath: "./sidebars.ts",
					remarkPlugins: [remarkMath],
					rehypePlugins: [rehypeKatex],
					// Please change this to your repo.
					// Remove this to remove the "edit this page" links.
					editUrl: "https://github.com/denys/rate-limiter/tree/main/website/",
				},
				blog: false,
				theme: {
					customCss: "./src/css/custom.css",
				},
			} satisfies Preset.Options,
		],
	],

	stylesheets: [
		{
			href: "https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css",
			type: "text/css",
			integrity: "sha384-n8MVd4RsNIU0tAv4ct0nTaAbDJwPJzDEaqSD1odI+WdtXRGWt2kTvGFasHpSy3SV",
			crossorigin: "anonymous",
		},
	],

	themeConfig: {
		// Replace with your project's social card
		image: "img/docusaurus-social-card.jpg",
		colorMode: {
			respectPrefersColorScheme: true,
		},
		navbar: {
			title: "Rate Limiter",
			logo: {
				alt: "Rate Limiter Logo",
				src: "img/logo.svg",
			},
			items: [
				{
					type: "docSidebar",
					sidebarId: "tutorialSidebar",
					position: "left",
					label: "Documentation",
				},
				{
					href: "https://github.com/denys/rate-limiter",
					label: "GitHub",
					position: "right",
				},
			],
		},
		footer: {
			style: "dark",
			links: [
				{
					title: "Docs",
					items: [
						{
							label: "Architecture Overview",
							to: "/docs/architecture/overview",
						},
						{
							label: "Algorithms Reference",
							to: "/docs/library/algorithms",
						},
					],
				},
				{
					title: "More",
					items: [
						{
							label: "GitHub",
							href: "https://github.com/denys/rate-limiter",
						},
						{
							label: "Personal Website",
							href: "https://bushuliak.com",
						},
					],
				},
			],
			copyright: `Copyright © ${new Date().getFullYear()} Denys Bushuliak. Built with Docusaurus.`,
		},
		prism: {
			theme: prismThemes.github,
			darkTheme: prismThemes.dracula,
			additionalLanguages: ["rust"],
		},
	} satisfies Preset.ThemeConfig,
};

export default config;

