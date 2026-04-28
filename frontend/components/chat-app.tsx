"use client";

import { FormEvent, useMemo, useState } from "react";
import { QueryClient, QueryClientProvider, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Menu, MessageSquare, Plus, Send, X } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  ApiError,
  ChatMessage,
  Conversation,
  createConversation,
  listConversations,
  listMessages,
  streamAssistantMessage
} from "@/lib/api";

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: 1,
        refetchOnWindowFocus: false
      }
    }
  });
}

export function ChatApp() {
  const [queryClient] = useState(makeQueryClient);

  return (
    <QueryClientProvider client={queryClient}>
      <ChatWorkspace />
    </QueryClientProvider>
  );
}

function ChatWorkspace() {
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [optimisticMessages, setOptimisticMessages] = useState<ChatMessage[]>([]);
  const [streamingAssistantId, setStreamingAssistantId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const conversationsQuery = useQuery({
    queryKey: ["conversations"],
    queryFn: listConversations
  });
  const activeConversationId = selectedId ?? conversationsQuery.data?.[0]?.id ?? null;

  const messagesQuery = useQuery({
    queryKey: ["messages", activeConversationId],
    queryFn: () => listMessages(activeConversationId as string),
    enabled: Boolean(activeConversationId)
  });

  const createMutation = useMutation({
    mutationFn: createConversation,
    onSuccess: (conversation) => {
      queryClient.setQueryData<Conversation[]>(["conversations"], (current = []) => [
        conversation,
        ...current.filter((item) => item.id !== conversation.id)
      ]);
      setSelectedId(conversation.id);
      setOptimisticMessages([]);
      setIsSidebarOpen(false);
    }
  });

  const messages = useMemo(() => {
    return [...(messagesQuery.data ?? []), ...optimisticMessages];
  }, [messagesQuery.data, optimisticMessages]);

  const selectedConversation = conversationsQuery.data?.find(
    (conversation) => conversation.id === activeConversationId
  );
  const isSending = Boolean(streamingAssistantId);

  async function startNewConversation() {
    setError(null);
    setOptimisticMessages([]);
    await createMutation.mutateAsync("New chat");
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const content = draft.trim();
    if (!content || isSending) {
      return;
    }

    setError(null);
    setDraft("");

    try {
      let conversationId = activeConversationId;
      if (!conversationId) {
        const conversation = await createMutation.mutateAsync("New chat");
        conversationId = conversation.id;
      }

      const userMessage = makeLocalMessage(conversationId, "user", content);
      const assistantMessage = makeLocalMessage(conversationId, "assistant", "");
      setStreamingAssistantId(assistantMessage.id);
      setOptimisticMessages((current) => [...current, userMessage, assistantMessage]);

      await streamAssistantMessage(conversationId, content, (chunk) => {
        setOptimisticMessages((current) =>
          current.map((message) =>
            message.id === assistantMessage.id
              ? { ...message, content: `${message.content}${chunk}` }
              : message
          )
        );
      });

      await queryClient.invalidateQueries({ queryKey: ["conversations"] });
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : "The assistant request failed");
    } finally {
      setStreamingAssistantId(null);
    }
  }

  return (
    <main className="flex min-h-screen bg-[#f7f7f4] text-ink">
      <button
        type="button"
        className="fixed left-3 top-3 z-30 grid h-10 w-10 place-items-center rounded-md border border-line bg-white shadow-soft md:hidden"
        onClick={() => setIsSidebarOpen(true)}
        aria-label="Open conversations"
      >
        <Menu className="h-5 w-5" aria-hidden="true" />
      </button>

      <ConversationSidebar
        conversations={conversationsQuery.data ?? []}
        selectedId={activeConversationId}
        loading={conversationsQuery.isLoading}
        open={isSidebarOpen}
        onClose={() => setIsSidebarOpen(false)}
        onSelect={(id) => {
          setSelectedId(id);
          setOptimisticMessages([]);
          setIsSidebarOpen(false);
        }}
        onNew={startNewConversation}
      />

      <section className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 shrink-0 items-center justify-between border-b border-line bg-white/90 px-4 pl-16 md:px-6 md:pl-6">
          <div className="min-w-0">
            <h1 className="truncate text-base font-semibold">gpt-copy-v5</h1>
            <p className="truncate text-sm text-neutral-500">
              {selectedConversation?.title ?? "New chat"}
            </p>
          </div>
          <button
            type="button"
            className="hidden h-10 w-10 place-items-center rounded-md border border-line text-neutral-600 hover:bg-panel md:grid"
            onClick={startNewConversation}
            aria-label="New chat"
          >
            <Plus className="h-5 w-5" aria-hidden="true" />
          </button>
        </header>

        <div className="flex-1 overflow-y-auto px-4 py-6 md:px-8">
          <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
            {messages.length === 0 && (
              <div className="rounded-md border border-dashed border-line bg-white p-6 text-center text-sm text-neutral-500">
                No messages yet
              </div>
            )}
            {messages.map((message) => (
              <MessageBubble
                key={message.id}
                message={message}
                streaming={message.id === streamingAssistantId}
              />
            ))}
            {messagesQuery.isLoading && (
              <div className="rounded-md border border-line bg-white p-4 text-sm text-neutral-500">
                Loading conversation...
              </div>
            )}
          </div>
        </div>

        <div className="border-t border-line bg-white px-4 py-4 md:px-8">
          <div className="mx-auto max-w-3xl">
            {error && (
              <div role="alert" className="mb-3 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
                {error}
              </div>
            )}
            <form onSubmit={handleSubmit} className="flex items-end gap-3 rounded-md border border-line bg-[#fbfbf8] p-2 shadow-soft">
              <label htmlFor="composer" className="sr-only">
                Message
              </label>
              <textarea
                id="composer"
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    event.currentTarget.form?.requestSubmit();
                  }
                }}
                placeholder="Message gpt-copy-v5"
                className="max-h-40 min-h-12 flex-1 resize-none border-0 bg-transparent px-3 py-3 text-sm outline-none ring-0 focus:ring-0"
                disabled={isSending}
              />
              <button
                type="submit"
                disabled={!draft.trim() || isSending}
                className="grid h-10 w-10 shrink-0 place-items-center rounded-md bg-accent text-white transition hover:bg-teal-800 disabled:cursor-not-allowed disabled:bg-neutral-300"
                aria-label="Send message"
              >
                <Send className="h-4 w-4" aria-hidden="true" />
              </button>
            </form>
          </div>
        </div>
      </section>
    </main>
  );
}

function ConversationSidebar({
  conversations,
  selectedId,
  loading,
  open,
  onClose,
  onSelect,
  onNew
}: {
  conversations: Conversation[];
  selectedId: string | null;
  loading: boolean;
  open: boolean;
  onClose: () => void;
  onSelect: (id: string) => void;
  onNew: () => void;
}) {
  return (
    <>
      <div
        className={`fixed inset-0 z-40 bg-black/30 transition md:hidden ${open ? "block" : "hidden"}`}
        onClick={onClose}
      />
      <aside
        className={`fixed inset-y-0 left-0 z-50 flex w-80 max-w-[86vw] flex-col border-r border-line bg-[#272a2a] text-white transition-transform md:static md:z-auto md:w-72 md:translate-x-0 ${
          open ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <div className="flex h-16 items-center justify-between border-b border-white/10 px-4">
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">gpt-copy-v5</div>
            <div className="truncate text-xs text-white/55">Conversations</div>
          </div>
          <button
            type="button"
            className="grid h-9 w-9 place-items-center rounded-md text-white/75 hover:bg-white/10 md:hidden"
            onClick={onClose}
            aria-label="Close conversations"
          >
            <X className="h-5 w-5" aria-hidden="true" />
          </button>
        </div>
        <div className="p-3">
          <button
            type="button"
            onClick={onNew}
            className="flex h-11 w-full items-center gap-2 rounded-md border border-white/15 px-3 text-sm text-white transition hover:bg-white/10"
          >
            <Plus className="h-4 w-4" aria-hidden="true" />
            <span>New chat</span>
          </button>
        </div>
        <nav className="min-h-0 flex-1 overflow-y-auto px-3 pb-4">
          {loading && <div className="px-3 py-2 text-sm text-white/55">Loading...</div>}
          {conversations.map((conversation) => (
            <button
              key={conversation.id}
              type="button"
              onClick={() => onSelect(conversation.id)}
              className={`mb-1 flex h-11 w-full items-center gap-2 rounded-md px-3 text-left text-sm transition ${
                conversation.id === selectedId ? "bg-white text-ink" : "text-white/80 hover:bg-white/10"
              }`}
            >
              <MessageSquare className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span className="truncate">{conversation.title}</span>
            </button>
          ))}
        </nav>
      </aside>
    </>
  );
}

function MessageBubble({ message, streaming }: { message: ChatMessage; streaming: boolean }) {
  const isUser = message.role === "user";

  return (
    <article className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[88%] rounded-md px-4 py-3 text-sm shadow-sm md:max-w-[78%] ${
          isUser ? "bg-accent text-white" : "border border-line bg-white text-ink"
        }`}
      >
        {isUser ? (
          <p className="whitespace-pre-wrap">{message.content}</p>
        ) : (
          <div className="markdown-body">
            {message.content ? (
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
            ) : (
              <span className="text-neutral-500">{streaming ? "Thinking..." : ""}</span>
            )}
          </div>
        )}
      </div>
    </article>
  );
}

function makeLocalMessage(
  conversationId: string,
  role: ChatMessage["role"],
  content: string
): ChatMessage {
  return {
    id: crypto.randomUUID(),
    conversation_id: conversationId,
    role,
    content,
    created_at: new Date().toISOString()
  };
}
