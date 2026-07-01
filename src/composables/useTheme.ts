import { ref } from "vue";

type Theme = "light" | "dark";

const theme = ref<Theme>((localStorage.getItem("theme") as Theme) || "light");

export function useTheme() {
  function setTheme(newTheme: Theme) {
    theme.value = newTheme;
    localStorage.setItem("theme", newTheme);
    applyTheme(newTheme);
  }

  function applyTheme(t: Theme) {
    const html = document.documentElement;
    if (t === "dark") {
      html.classList.add("dark");
    } else {
      html.classList.remove("dark");
    }
  }

  applyTheme(theme.value);

  return {
    theme,
    setTheme,
  };
}
