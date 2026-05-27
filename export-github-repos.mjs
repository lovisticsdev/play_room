// export-github-repos.mjs
import fs from "node:fs/promises";

const username = process.argv[2] ?? "lovisticsdev";
const token = process.env.GITHUB_TOKEN;

const headers = {
  Accept: "application/vnd.github+json",
  "User-Agent": "repo-export-script",
};

if (token) {
  headers.Authorization = `Bearer ${token}`;
}

async function githubJson(url) {
  const res = await fetch(url, { headers });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${res.statusText}: ${url}\n${text}`);
  }

  return res.json();
}

async function fetchAllRepos(username) {
  const repos = [];
  let page = 1;

  while (true) {
    const url =
      `https://api.github.com/users/${username}/repos` +
      `?type=owner&sort=updated&direction=desc&per_page=100&page=${page}`;

    const batch = await githubJson(url);

    if (!Array.isArray(batch) || batch.length === 0) {
      break;
    }

    repos.push(...batch);
    page += 1;
  }

  return repos;
}

async function fetchReadme(owner, repo) {
  const url = `https://api.github.com/repos/${owner}/${repo}/readme`;

  try {
    const readme = await githubJson(url);
    const content = Buffer.from(readme.content ?? "", "base64").toString("utf8");

    return {
      name: readme.name,
      path: readme.path,
      size: readme.size,
      content,
    };
  } catch {
    return null;
  }
}

async function fetchLanguages(owner, repo) {
  const url = `https://api.github.com/repos/${owner}/${repo}/languages`;

  try {
    return await githubJson(url);
  } catch {
    return {};
  }
}

async function main() {
  const rawRepos = await fetchAllRepos(username);

  const repos = [];

  for (const repo of rawRepos) {
    const [languages, readme] = await Promise.all([
      fetchLanguages(repo.owner.login, repo.name),
      fetchReadme(repo.owner.login, repo.name),
    ]);

    repos.push({
      name: repo.name,
      full_name: repo.full_name,
      description: repo.description,
      html_url: repo.html_url,
      language: repo.language,
      languages,
      visibility: repo.visibility,
      archived: repo.archived,
      fork: repo.fork,
      created_at: repo.created_at,
      updated_at: repo.updated_at,
      pushed_at: repo.pushed_at,
      readme,
    });
  }

  const exportData = {
    exported_at: new Date().toISOString(),
    username,
    repo_count: repos.length,
    repos,
  };

  const outputPath = `github-repos-${username}.json`;
  await fs.writeFile(outputPath, JSON.stringify(exportData, null, 2), "utf8");

  console.log(`Exported ${repos.length} repos to ${outputPath}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});