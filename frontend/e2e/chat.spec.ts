import { expect, test } from "@playwright/test";

const conversation = {
  id: "00000000-0000-4000-8000-000000000001",
  title: "New chat",
  created_at: "2026-04-28T00:00:00Z",
  updated_at: "2026-04-28T00:00:00Z"
};

test("sends one message through the chat composer", async ({ page }) => {
  let created = false;

  await page.route("http://127.0.0.1:3999/**", async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();

    if (url.endsWith("/api/conversations") && method === "GET") {
      await route.fulfill({ json: { conversations: created ? [conversation] : [] } });
      return;
    }
    if (url.endsWith("/api/conversations") && method === "POST") {
      created = true;
      await route.fulfill({ json: { conversation } });
      return;
    }
    if (url.endsWith(`/api/conversations/${conversation.id}/messages`) && method === "GET") {
      await route.fulfill({ json: { messages: [] } });
      return;
    }
    if (url.endsWith(`/api/conversations/${conversation.id}/stream`) && method === "POST") {
      await route.fulfill({
        status: 200,
        headers: { "content-type": "text/event-stream" },
        body: "event: chunk\ndata: Hello from Playwright\n\nevent: done\ndata: {}\n\n"
      });
      return;
    }

    await route.fulfill({
      status: 404,
      json: { error: { code: "not_found", message: `Unhandled ${method} ${url}` } }
    });
  });

  await page.goto("/");
  await page.getByPlaceholder("Message gpt-copy-v5").fill("Hello");
  await page.getByRole("button", { name: "Send message" }).click();

  await expect(page.getByText("Hello", { exact: true })).toBeVisible();
  await expect(page.getByText("Hello from Playwright")).toBeVisible();
});
