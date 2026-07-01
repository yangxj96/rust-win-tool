import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/services",
    },
    {
      path: "/services",
      name: "Services",
      component: () => import("../views/ServiceManagement.vue"),
      meta: { title: "menu.services" },
    },
    {
      path: "/settings",
      name: "Settings",
      component: () => import("../views/Settings.vue"),
      meta: { title: "menu.settings" },
    },
  ],
});

export default router;
