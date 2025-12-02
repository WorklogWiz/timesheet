/// <reference types="svelte" />
/// <reference types="vite/client" />

// Allow CSS imports
declare module '*.css' {
  const content: string;
  export default content;
}



