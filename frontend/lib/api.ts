import { z } from "zod";

const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:3001";

const conversationSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string()
});

const messageSchema = z.object({
  id: z.string().uuid(),
  conversation_id: z.string().uuid(),
  role: z.enum(["system", "user", "assistant"]),
  content: z.string(),
  created_at: z.string()
});

const conversationsResponseSchema = z.object({
  conversations: z.array(conversationSchema)
});

const conversationResponseSchema = z.object({
  conversation: conversationSchema
});

const messagesResponseSchema = z.object({
  messages: z.array(messageSchema)
});

const errorResponseSchema = z.object({
  error: z.object({
    code: z.string(),
    message: z.string()
  })
});

export type Conversation = z.infer<typeof conversationSchema>;
export type ChatMessage = z.infer<typeof messageSchema>;

export class ApiError extends Error {
  code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

export async function listConversations(): Promise<Conversation[]> {
  const data = await requestJson(`${API_BASE_URL}/api/conversations`);
  return conversationsResponseSchema.parse(data).conversations;
}

export async function createConversation(title?: string): Promise<Conversation> {
  const data = await requestJson(`${API_BASE_URL}/api/conversations`, {
    method: "POST",
    body: JSON.stringify({ title })
  });
  return conversationResponseSchema.parse(data).conversation;
}

export async function listMessages(conversationId: string): Promise<ChatMessage[]> {
  const data = await requestJson(`${API_BASE_URL}/api/conversations/${conversationId}/messages`);
  return messagesResponseSchema.parse(data).messages;
}

export async function streamAssistantMessage(
  conversationId: string,
  content: string,
  onChunk: (chunk: string) => void
): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/api/conversations/${conversationId}/stream`, {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify({ content })
  });

  if (!response.ok) {
    throw await parseApiError(response);
  }
  if (!response.body) {
    throw new ApiError("stream_error", "The response stream was empty");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    buffer += decoder.decode(value, { stream: true });

    let index = buffer.indexOf("\n\n");
    while (index >= 0) {
      const frame = buffer.slice(0, index);
      buffer = buffer.slice(index + 2);
      handleSseFrame(frame, onChunk);
      index = buffer.indexOf("\n\n");
    }
  }
}

async function requestJson(url: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(url, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...init?.headers
    }
  });
  if (!response.ok) {
    throw await parseApiError(response);
  }
  return response.json();
}

async function parseApiError(response: Response): Promise<ApiError> {
  const fallback = new ApiError("request_failed", `Request failed with ${response.status}`);
  try {
    const body = errorResponseSchema.parse(await response.json());
    return new ApiError(body.error.code, body.error.message);
  } catch {
    return fallback;
  }
}

function handleSseFrame(frame: string, onChunk: (chunk: string) => void) {
  let event = "message";
  const data: string[] = [];

  for (const line of frame.split("\n")) {
    if (line.startsWith("event:")) {
      event = line.slice("event:".length).trim();
    }
    if (line.startsWith("data:")) {
      const value = line.slice("data:".length);
      data.push(value.startsWith(" ") ? value.slice(1) : value);
    }
  }

  const payload = data.join("\n");
  if (event === "error") {
    throw new ApiError("stream_error", payload || "The assistant stream failed");
  }
  if (event === "chunk") {
    onChunk(payload);
  }
}
