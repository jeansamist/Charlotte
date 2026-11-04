"use client";
import {
  AlertCircleIcon,
  Delete02Icon,
  Key01Icon,
  MoreVerticalIcon,
  Shield01Icon,
  UserAdd01Icon,
  UserIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { PageHeader } from "@/components/page-header";
import { useUser } from "@/components/session";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api, type Role, type User } from "@/lib/api";
import { attempt } from "@/lib/errors";
import { usePoll } from "@/lib/hooks";

export default function UsersPage() {
  const me = useUser();
  const list = usePoll<{ users: User[] }>("/api/users");
  const [resetFor, setResetFor] = useState<User>();
  const [deleteFor, setDeleteFor] = useState<User>();

  if (me.role !== "admin")
    return (
      <Alert variant="destructive">
        <HugeiconsIcon icon={AlertCircleIcon} strokeWidth={2} />
        <AlertTitle>Administrators only</AlertTitle>
      </Alert>
    );

  const toggleRole = (u: User) =>
    attempt(async () => {
      await api(`/api/users/${u.id}`, { method: "PATCH", body: { role: u.role === "admin" ? "member" : "admin" } });
      await list.reload();
    }, `${u.username} is now ${u.role === "admin" ? "a member" : "an administrator"}`);

  const remove = (u: User) =>
    attempt(async () => {
      await api(`/api/users/${u.id}`, { method: "DELETE" });
      await list.reload();
    }, `${u.username} deleted`);

  return (
    <>
      <PageHeader title="Users" description="People who can sign in to Charlotte." actions={<CreateUserDialog onCreated={list.reload} />} />

      <Card className="py-0">
        <CardContent className="px-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="pl-6">User</TableHead>
                <TableHead>Role</TableHead>
                <TableHead className="hidden sm:table-cell">Created</TableHead>
                <TableHead className="w-12 pr-6" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {!list.data &&
                [0, 1].map((i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={4} className="px-6">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}
              {list.data?.users.map((u) => (
                <TableRow key={u.id}>
                  <TableCell className="pl-6">
                    <div className="flex items-center gap-3">
                      <Avatar className="size-8">
                        <AvatarFallback>{(u.displayName || u.username).slice(0, 2).toUpperCase()}</AvatarFallback>
                      </Avatar>
                      <div className="leading-tight">
                        <div className="font-medium">
                          {u.displayName || u.username}
                          {u.id === me.id && <span className="ml-1.5 text-xs text-muted-foreground">(you)</span>}
                        </div>
                        <div className="text-xs text-muted-foreground">@{u.username}</div>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={u.role === "admin" ? "default" : "secondary"}>{u.role}</Badge>
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground sm:table-cell">
                    {u.createdAt ? new Date(u.createdAt).toLocaleDateString() : "—"}
                  </TableCell>
                  <TableCell className="pr-6 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon" aria-label={`Actions for ${u.username}`}>
                          <HugeiconsIcon icon={MoreVerticalIcon} strokeWidth={2} />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onSelect={() => setResetFor(u)}>
                          <HugeiconsIcon icon={Key01Icon} strokeWidth={2} /> Reset password
                        </DropdownMenuItem>
                        {u.id !== me.id && (
                          <>
                            <DropdownMenuItem onSelect={() => void toggleRole(u)}>
                              <HugeiconsIcon icon={u.role === "admin" ? UserIcon : Shield01Icon} strokeWidth={2} />
                              Make {u.role === "admin" ? "member" : "administrator"}
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem variant="destructive" onSelect={() => setDeleteFor(u)}>
                              <HugeiconsIcon icon={Delete02Icon} strokeWidth={2} /> Delete
                            </DropdownMenuItem>
                          </>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <ResetPasswordDialog user={resetFor} onClose={() => setResetFor(undefined)} />

      <AlertDialog open={!!deleteFor} onOpenChange={(o) => !o && setDeleteFor(undefined)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete {deleteFor?.username}?</AlertDialogTitle>
            <AlertDialogDescription>The account and its sessions are removed. This cannot be undone.</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => deleteFor && void remove(deleteFor)}>
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

function CreateUserDialog({ onCreated }: { onCreated: () => Promise<void> }) {
  const empty = { username: "", displayName: "", password: "", role: "member" as Role };
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState(empty);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    const ok = await attempt(async () => {
      await api("/api/users", { method: "POST", body: form });
      await onCreated();
    }, `User ${form.username} created`);
    if (ok) {
      setForm(empty);
      setOpen(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <HugeiconsIcon icon={UserAdd01Icon} strokeWidth={2} />
          Add user
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form onSubmit={submit} className="space-y-6">
          <DialogHeader>
            <DialogTitle>Add a user</DialogTitle>
            <DialogDescription>They sign in with this username and password.</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="new-username">Username</FieldLabel>
              <Input id="new-username" required value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} />
              <FieldDescription>3–32 characters: letters, digits, . _ -</FieldDescription>
            </Field>
            <Field>
              <FieldLabel htmlFor="new-display">Display name</FieldLabel>
              <Input id="new-display" value={form.displayName} onChange={(e) => setForm({ ...form, displayName: e.target.value })} />
            </Field>
            <Field>
              <FieldLabel htmlFor="new-password">Password</FieldLabel>
              <Input
                id="new-password"
                type="password"
                autoComplete="new-password"
                minLength={10}
                required
                value={form.password}
                onChange={(e) => setForm({ ...form, password: e.target.value })}
              />
              <FieldDescription>At least 10 characters.</FieldDescription>
            </Field>
            <Field>
              <FieldLabel>Role</FieldLabel>
              <Select value={form.role} onValueChange={(v) => setForm({ ...form, role: v as Role })}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="member">Member</SelectItem>
                  <SelectItem value="admin">Administrator</SelectItem>
                </SelectContent>
              </Select>
            </Field>
          </FieldGroup>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit">Create</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function ResetPasswordDialog({ user, onClose }: { user?: User; onClose: () => void }) {
  const [password, setPassword] = useState("");

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!user) return;
    const ok = await attempt(
      () => api(`/api/users/${user.id}`, { method: "PATCH", body: { password } }),
      `Password of ${user.username} changed`,
    );
    if (ok) {
      setPassword("");
      onClose();
    }
  }

  return (
    <Dialog open={!!user} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <form onSubmit={submit} className="space-y-6">
          <DialogHeader>
            <DialogTitle>Reset password</DialogTitle>
            <DialogDescription>{user?.username} will be signed out everywhere.</DialogDescription>
          </DialogHeader>
          <Field>
            <FieldLabel htmlFor="reset-password">New password</FieldLabel>
            <Input
              id="reset-password"
              type="password"
              autoComplete="new-password"
              minLength={10}
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            <FieldDescription>At least 10 characters.</FieldDescription>
          </Field>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit">Change password</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
