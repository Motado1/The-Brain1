export interface QueryRequest {
  question: string;
}

export interface QueryResponse {
  answer: string;
  sources: Array<{
    id: string;
    snippet: string;
  }>;
}