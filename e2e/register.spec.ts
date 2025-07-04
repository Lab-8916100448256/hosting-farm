import { test, expect } from "@playwright/test";

test.describe.configure({ mode: "serial" });

let pageMr: Page;
let pageMs: Page;
let pageDummy: Page;
let pageMail: Page;

test.beforeAll(async ({ browser }) => {
  pageMr = await browser.newPage();
  pageMs = await browser.newPage();
  pageDummy = await browser.newPage();
  pageMail = await browser.newPage();
});

test.afterAll(async () => {
  await pageMr.close();
  await pageMs.close();
  await pageDummy.close();
  await pageMail.close();
});

test.describe("basic register flow", () => {
  test("sign up Mr", async () => {
    await pageMr.goto("http://localhost:5151/");
    await expect(
      pageMr.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await pageMr.getByRole("link", { name: "Sign up" }).click();
    await pageMr.getByRole("textbox", { name: "Full name" }).fill("Mr Test");
    await pageMr
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await pageMr
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageMr
      .getByRole("textbox", { name: "Confirm password" })
      .fill("Test!");
    await pageMr.getByRole("button", { name: "Create account" }).click();
    await expect(pageMr.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });

  test("confirm email", async () => {
    await pageMail.goto("http://localhost:1080/#/");
    await expect(pageMail.locator("body")).toContainText(
      "Welcome to Hosting Farm Mr Test To: mr.test@example.com"
    );
    await pageMail
      .getByRole("link", { name: "Welcome to Hosting Farm Mr" })
      .nth(0)
      .click();

    // For some reasons, this is not working. When running playwright tests, the email confirmation page opens inside the iframe of the mail app instead of in a popup as it should.
    /*
    const page1Promise = pageMail.waitForEvent("popup");
    await pageMail
      .locator("iframe")
      .first()
      .contentFrame()
      .getByRole("link", { name: "Verify Your Account" })
      .click({ button: "middle" });
    const page1 = await page1Promise;
    await expect(page1.locator("h3")).toContainText(
      "Email Verified Successfully"
    );
    await page1.getByRole("link", { name: "Go to Profile" }).click();
    */

    // So we navigate to the email confirmation link like this
    await pageMr.goto(
      await pageMail
        .locator("iframe")
        .first()
        .contentFrame()
        .getByRole("link", { name: "Verify Your Account" })
        .getAttribute("href")
    );
    await expect(pageMr.locator("h3")).toContainText(
      "Email Verified Successfully"
    );
    await pageMr.getByRole("link", { name: "Go to Profile" }).click();
  });

  test("log in", async () => {
    await pageMr.goto("http://localhost:5151/auth/login");
    await pageMr
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await pageMr
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageMr.getByRole("button", { name: "Sign in" }).click();
  });

  test("logout", async () => {
    await pageMr.getByRole("button", { name: "Open user menu Mr" }).click();
    await pageMr.getByRole("menuitem", { name: "Logout" }).click();
  });
});

test.describe("Register dummy users", () => {
  test("sign up dymmy 1", async () => {
    await pageDummy.goto("http://localhost:5151/");
    await expect(
      pageDummy.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await pageDummy.getByRole("link", { name: "Sign up" }).click();
    await pageDummy.getByRole("textbox", { name: "Full name" }).fill("Dummy 1");
    await pageDummy
      .getByRole("textbox", { name: "Email address" })
      .fill("dummy.1@example.com");
    await pageDummy
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageDummy
      .getByRole("textbox", { name: "Confirm password" })
      .fill("Test!");
    await pageDummy.getByRole("button", { name: "Create account" }).click();
    await expect(pageDummy.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });
  test("sign up dymmy 2", async () => {
    await pageDummy.goto("http://localhost:5151/");
    await expect(
      pageDummy.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await pageDummy.getByRole("link", { name: "Sign up" }).click();
    await pageDummy.getByRole("textbox", { name: "Full name" }).fill("Dummy 2");
    await pageDummy
      .getByRole("textbox", { name: "Email address" })
      .fill("dummy.2@example.com");
    await pageDummy
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageDummy
      .getByRole("textbox", { name: "Confirm password" })
      .fill("Test!");
    await pageDummy.getByRole("button", { name: "Create account" }).click();
    await expect(pageDummy.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });
  test("sign up dymmy 3", async () => {
    await pageDummy.goto("http://localhost:5151/");
    await expect(
      pageDummy.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await pageDummy.getByRole("link", { name: "Sign up" }).click();
    await pageDummy.getByRole("textbox", { name: "Full name" }).fill("Dummy 3");
    await pageDummy
      .getByRole("textbox", { name: "Email address" })
      .fill("dummy.3@example.com");
    await pageDummy
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageDummy
      .getByRole("textbox", { name: "Confirm password" })
      .fill("Test!");
    await pageDummy.getByRole("button", { name: "Create account" }).click();
    await expect(pageDummy.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });
});

test.describe("Teams management", () => {
  test("log in", async () => {
    await pageMr.goto("http://localhost:5151/auth/login");
    await pageMr
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await pageMr
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageMr.getByRole("button", { name: "Sign in" }).click();
  });

  test("edit admin team", async () => {
    await pageMr.getByRole("link", { name: "View your teams" }).click();
    await expect(pageMr.getByRole("main")).toContainText(
      "Default administrators team created automatically."
    );
    await pageMr.getByRole("link", { name: "View details" }).click();
    await pageMr.getByRole("link", { name: "Edit Team" }).click();
    await pageMr.getByRole("textbox", { name: "Description" }).click();
    await pageMr
      .getByRole("textbox", { name: "Description" })
      .press("ControlOrMeta+a");
    await pageMr
      .getByRole("textbox", { name: "Description" })
      .fill("Application administrators");
    await pageMr.getByRole("button", { name: "Save" }).click();
    await expect(pageMr.getByRole("main")).toContainText(
      "Application administrators"
    );
  });

  test("create test team", async () => {
    await pageMr.getByRole("link", { name: "Teams" }).click();
    await pageMr.getByRole("link", { name: "Create New Team" }).click();
    await pageMr.getByRole("textbox", { name: "Team Name" }).click();
    await pageMr.getByRole("textbox", { name: "Team Name" }).fill("Test team");
    await pageMr.getByRole("textbox", { name: "Description" }).click();
    await pageMr
      .getByRole("textbox", { name: "Description" })
      .fill("End to end tests team");
    await pageMr.getByRole("button", { name: "Create" }).click();
    await expect(pageMr.getByRole("main")).toContainText("Test team");
    await expect(pageMr.getByRole("main")).toContainText(
      "End to end tests team"
    );
    await expect(pageMr.getByRole("listitem")).toMatchAriaSnapshot(
      `- listitem: Owner Mr Mr Test mr.test@example.com`
    );
    await pageMr.getByRole("link", { name: "Teams" }).click();
    await expect(pageMr.getByRole("main")).toContainText("Administrators");
    await expect(pageMr.getByRole("main")).toContainText("Test team");
  });

  test("sign up Ms", async () => {
    await pageMs.goto("http://localhost:5151/");
    await expect(
      pageMs.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await pageMs.getByRole("link", { name: "Sign up" }).click();
    await pageMs.getByRole("textbox", { name: "Full name" }).fill("Ms Test");
    await pageMs
      .getByRole("textbox", { name: "Email address" })
      .fill("ms.test@example.com");
    await pageMs
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageMs
      .getByRole("textbox", { name: "Confirm password" })
      .fill("Test!");
    await pageMs.getByRole("button", { name: "Create account" }).click();
    await expect(pageMs.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });

  test("log in Ms", async () => {
    await pageMs.goto("http://localhost:5151/auth/login");
    await pageMs
      .getByRole("textbox", { name: "Email address" })
      .fill("ms.test@example.com");
    await pageMs
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await pageMs.getByRole("button", { name: "Sign in" }).click();
  });

  test("create test team 2", async () => {
    await pageMs.getByRole("link", { name: "Teams", exact: true }).click();
    await expect(pageMs.getByRole("main")).toContainText(
      "You don't have any teams yet."
    );
    await pageMs.getByRole("link", { name: "Create New Team" }).click();
    await pageMs.getByRole("textbox", { name: "Team Name" }).click();
    await pageMs
      .getByRole("textbox", { name: "Team Name" })
      .fill("Test team 2");
    await pageMs.getByRole("textbox", { name: "Description" }).click();
    await pageMs.getByRole("textbox", { name: "Description" }).click();
    await pageMs
      .getByRole("textbox", { name: "Description" })
      .fill("End to end tests team #2");
    await pageMs.getByRole("button", { name: "Create" }).click();
    await pageMs.getByRole("link", { name: "Invite Member" }).click();
    await pageMs.getByRole("textbox", { name: "User name" }).click();
    await pageMs
      .getByRole("textbox", { name: "User name" })
      .pressSequentially("mr", { delay: 100 });
    await expect(pageMs.getByRole("listitem")).toContainText("Mr Test");
    await pageMs.getByText("Mr Test").click();
    await pageMs.getByRole("button", { name: "Send Invitation" }).click();
    await expect(pageMs.getByRole("list")).toContainText(
      "Invited Mr Mr Test mr.test@example.com Cancel Invitation"
    );
  });

  test("Accept team invitation", async () => {
    await expect(
      pageMr.getByRole("link", { name: "Invitations" })
    ).toBeVisible();
    await pageMr.getByRole("link", { name: "Invitations" }).click();
    await expect(pageMr.getByRole("listitem")).toBeVisible();
    await expect(pageMr.locator("h3")).toContainText("Test team 2");
    await pageMr.getByRole("button", { name: "Accept" }).click();
    await pageMr.getByRole("link", { name: "Teams" }).click();
    await expect(pageMr.getByRole("main")).toContainText("Test team 2");
  });

});

test.describe("Cleanup emails", () => {
  test("delete all emails", async () => {
    await pageMail.goto("http://localhost:1080/#/");
    await pageMail.getByRole("link", { name: "" }).click();
    await pageMail.getByRole("link", { name: "" }).click();
    await pageMail.getByRole("link", { name: "" }).click();
    await pageMail.getByRole("link", { name: "" }).click();
  });
});

test.describe("Users management", () => {
  test("edit user", async () => {
    await pageMr.getByRole("link", { name: "Admin" }).click();
    await expect(pageMr.locator("#user-row-4")).toContainText("Dummy 3");
    await expect(pageMr.locator("#user-row-4")).toContainText(
      "dummy.3@example.com"
    );
    await pageMr
      .locator("#user-row-4")
      .getByRole("button", { name: "Edit" })
      .click();
    await pageMr.locator('input[name="name"]').fill("Dummy 3.0");
    await pageMr.locator('input[name="email"]').fill("dummy.3.0@example.com");
    await pageMr.getByRole("button", { name: "Save" }).click();
    await expect(pageMr.locator("#user-row-4")).toContainText("Dummy 3.0");
    await expect(pageMr.locator("#user-row-4")).toContainText(
      "dummy.3.0@example.com"
    );
  });
});
