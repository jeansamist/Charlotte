"use client";
import {
  BluetoothIcon,
  Logout03Icon,
  Settings02Icon,
  SlidersHorizontalIcon,
  UserCircleIcon,
  UserGroupIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { BrandMark } from "@/components/brand";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import type { User } from "@/lib/api";

type NavItem = { href: string; label: string; icon: IconSvgElement; admin?: boolean };

const CONTROL: NavItem[] = [
  { href: "/", label: "Audio", icon: SlidersHorizontalIcon },
  { href: "/bluetooth", label: "Bluetooth", icon: BluetoothIcon },
];
const ADMIN: NavItem[] = [
  { href: "/settings", label: "Settings", icon: Settings02Icon },
  { href: "/users", label: "Users", icon: UserGroupIcon, admin: true },
];

export function AppSidebar({ user, onSignOut }: { user: User; onSignOut: () => void }) {
  const pathname = usePathname();
  const group = (label: string, items: NavItem[]) => (
    <SidebarGroup>
      <SidebarGroupLabel>{label}</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {items
            .filter((i) => !i.admin || user.role === "admin")
            .map((i) => (
              <SidebarMenuItem key={i.href}>
                <SidebarMenuButton asChild isActive={pathname === i.href} tooltip={i.label}>
                  <Link href={i.href}>
                    <HugeiconsIcon icon={i.icon} strokeWidth={2} />
                    <span>{i.label}</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );

  const name = user.displayName || user.username;
  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild>
              <Link href="/">
                <BrandMark />
                <div className="grid flex-1 text-left leading-tight">
                  <span className="font-heading font-semibold">Charlotte</span>
                  <span className="text-xs text-muted-foreground">Home assistant</span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        {group("Control", CONTROL)}
        {group("Administration", ADMIN)}
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton asChild isActive={pathname === "/account"} size="lg" tooltip="My account">
              <Link href="/account">
                <Avatar className="size-8 rounded-lg">
                  <AvatarFallback className="rounded-lg">{name.slice(0, 2).toUpperCase()}</AvatarFallback>
                </Avatar>
                <div className="grid flex-1 text-left leading-tight">
                  <span className="truncate font-medium">{name}</span>
                  <span className="truncate text-xs text-muted-foreground">{user.role}</span>
                </div>
                <HugeiconsIcon icon={UserCircleIcon} strokeWidth={2} className="ml-auto" />
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton onClick={onSignOut} tooltip="Sign out">
              <HugeiconsIcon icon={Logout03Icon} strokeWidth={2} />
              <span>Sign out</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
