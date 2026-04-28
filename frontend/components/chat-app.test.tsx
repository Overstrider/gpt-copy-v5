import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ChatApp } from "./chat-app";

const conversation = {
  id: "00000000-0000-4000-8000-000000000001",
  title: "New chat",
  created_at: "2026-04-28T00:00:00Z",
  updated_at: "2026-04-28T00:00:00Z"
};

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ChatApp", () => {
  it("renders the empty chat shell", async () => {
    vi.spyOn(global, "fetch").mockResolvedValue(jsonResponse({ conversations: [] }));

    render(<ChatApp />);

    expect((await screen.findAllByText("gpt-copy-v5")).length).toBeGreaterThan(0);
    expect(await screen.findByText("No messages yet")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Message gpt-copy-v5")).toBeInTheDocument();
  });

  it("sends a message and renders streamed assistant content", async () => {
    let created = false;
    vi.spyOn(global, "fetch").mockImplementation(async (input, init) => {
      const url = input.toString();
      const method = init?.method ?? "GET";

      if (url.endsWith("/api/conversations") && method === "GET") {
        return jsonResponse({ conversations: created ? [conversation] : [] });
      }
      if (url.endsWith("/api/conversations") && method === "POST") {
        created = true;
        return jsonResponse({ conversation });
      }
      if (url.endsWith(`/api/conversations/${conversation.id}/messages`)) {
        return jsonResponse({ messages: [] });
      }
      if (url.endsWith(`/api/conversations/${conversation.id}/stream`) && method === "POST") {
        return streamResponse(["Hello ", "from mock"]);
      }

      return jsonResponse(
        { error: { code: "not_found", message: `Unhandled test URL: ${url}` } },
        404
      );
    });

    render(<ChatApp />);

    const composer = await screen.findByPlaceholderText("Message gpt-copy-v5");
    await userEvent.type(composer, "Hello model");
    await userEvent.click(screen.getByRole("button", { name: "Send message" }));

    expect(await screen.findByText("Hello model")).toBeInTheDocument();
    expect(await screen.findByText("Hello from mock")).toBeInTheDocument();
  });
});

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json"
    }
  });
}

function streamResponse(chunks: string[]): Response {
  const encoder = new TextEncoder();
  return new Response(
    new ReadableStream({
      start(controller) {
        for (const chunk of chunks) {
          controller.enqueue(encoder.encode(`event: chunk\ndata: ${chunk}\n\n`));
        }
        controller.enqueue(encoder.encode("event: done\ndata: {}\n\n"));
        controller.close();
      }
    }),
    {
      status: 200,
      headers: {
        "content-type": "text/event-stream"
      }
    }
  );
}
