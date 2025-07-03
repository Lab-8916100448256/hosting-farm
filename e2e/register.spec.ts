import { test, expect } from "@playwright/test";

test.describe.configure({ mode: "serial" });

let page: Page;

test.beforeAll(async ({ browser }) => {
  page = await browser.newPage();
});

test.afterAll(async () => {
  await page.close();
});

test.describe("basic register flow", () => {
  test("sign up", async () => {
    await page.goto("http://localhost:5151/");
    await expect(
      page.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await page.getByRole("link", { name: "Sign up" }).click();
    await page.getByRole("textbox", { name: "Full name" }).fill("Mr Test");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("textbox", { name: "Confirm password" }).fill("Test!");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });

  test("log in", async () => {
    await page.goto("http://localhost:5151/auth/login");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("button", { name: "Sign in" }).click();
  });

  test("confirm email", async () => {
    await page.goto("http://localhost:1080/#/");
    await expect(page.locator("body")).toContainText(
      "Welcome to Hosting Farm Mr Test To: mr.test@example.com"
    );
    await page
      .getByRole("link", { name: "Welcome to Hosting Farm Mr" })
      .nth(0)
      .click();

    // For some reasons, this is not working. It seems the email confirmation page opens inside the iframe of the mail app.
    /*
    const page1Promise = page.waitForEvent("popup");
    await page
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

    // So we navigate to it like this
    await page.goto(
      await page
        .locator("iframe")
        .first()
        .contentFrame()
        .getByRole("link", { name: "Verify Your Account" })
        .getAttribute("href")
    );
    await expect(page.locator("h3")).toContainText(
      "Email Verified Successfully"
    );
    await page.getByRole("link", { name: "Go to Profile" }).click();
  });
});

test.describe("Teams management", () => {
  test("log in", async () => {
    await page.goto("http://localhost:5151/auth/login");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("button", { name: "Sign in" }).click();
  });

  test("edit admin team", async () => {
    await page.getByRole("link", { name: "View your teams" }).click();
    await expect(page.getByRole("main")).toContainText(
      "Default administrators team created automatically."
    );
    await page.getByRole("link", { name: "View details" }).click();
    await page.getByRole("link", { name: "Edit Team" }).click();
    await page.getByRole("textbox", { name: "Description" }).click();
    await page
      .getByRole("textbox", { name: "Description" })
      .press("ControlOrMeta+a");
    await page
      .getByRole("textbox", { name: "Description" })
      .fill("Application administrators");
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByRole("main")).toContainText(
      "Application administrators"
    );
  });

  test("create test team", async () => {
    await page.getByRole("link", { name: "Teams" }).click();
    await page.getByRole("link", { name: "Create New Team" }).click();
    await page.getByRole("textbox", { name: "Team Name" }).click();
    await page.getByRole("textbox", { name: "Team Name" }).fill("Test team");
    await page.getByRole("textbox", { name: "Description" }).click();
    await page
      .getByRole("textbox", { name: "Description" })
      .fill("End to end tests team");
    await page.getByRole("button", { name: "Create" }).click();
    await expect(page.getByRole("main")).toContainText("Test team");
    await expect(page.getByRole("main")).toContainText("End to end tests team");
    await expect(page.getByRole("listitem")).toMatchAriaSnapshot(
      `- listitem: Owner Mr Mr Test mr.test@example.com`
    );
    await page.getByRole("link", { name: "Teams" }).click();
    await expect(page.getByRole("main")).toContainText("Administrators");
    await expect(page.getByRole("main")).toContainText("Test team");
  });

  test("logout", async () => {
    await page.getByRole("button", { name: "Open user menu Mr" }).click();
    await page.getByRole("menuitem", { name: "Logout" }).click();
  });

  test("sign up 2", async () => {
    await page.goto("http://localhost:5151/");
    await expect(
      page.getByRole("link", { name: "Hosting Farm" })
    ).toBeVisible();
    await page.getByRole("link", { name: "Sign up" }).click();
    await page.getByRole("textbox", { name: "Full name" }).fill("Ms Test");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("ms.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("textbox", { name: "Confirm password" }).fill("Test!");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByRole("main")).toContainText(
      "Sign in to your account"
    );
  });

  test("log in 2", async () => {
    await page.goto("http://localhost:5151/auth/login");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("ms.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("button", { name: "Sign in" }).click();
  });

  test("create test team 2", async () => {
    await page.getByRole("link", { name: "Teams", exact: true }).click();
    await expect(page.getByRole("main")).toContainText(
      "You don't have any teams yet."
    );
    await page.getByRole("link", { name: "Create New Team" }).click();
    await page.getByRole("textbox", { name: "Team Name" }).click();
    await page.getByRole("textbox", { name: "Team Name" }).fill("Test team 2");
    await page.getByRole("textbox", { name: "Description" }).click();
    await page.getByRole("textbox", { name: "Description" }).click();
    await page
      .getByRole("textbox", { name: "Description" })
      .fill("End to end tests team #2");
    await page.getByRole("button", { name: "Create" }).click();
    await page.getByRole("link", { name: "Invite Member" }).click();
    await page.getByRole("textbox", { name: "User name" }).click();
    await page
      .getByRole("textbox", { name: "User name" })
      .pressSequentially("mr", { delay: 100 });
    await expect(page.getByRole("listitem")).toContainText("Mr Test");
    await page.getByText("Mr Test").click();
    await page.getByRole("button", { name: "Send Invitation" }).click();
    await expect(page.getByRole("list")).toContainText(
      "Invited Mr Mr Test mr.test@example.com Cancel Invitation"
    );
  });

  test("logout 2", async () => {
    await page.getByRole("button", { name: "Open user menu Ms" }).click();
    await page.getByRole("menuitem", { name: "Logout" }).click();
  });

  test("log in 3", async () => {
    await page.goto("http://localhost:5151/auth/login");
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill("mr.test@example.com");
    await page
      .getByRole("textbox", { name: "Password", exact: true })
      .fill("Test!");
    await page.getByRole("button", { name: "Sign in" }).click();
  });

  test("Accept team invitation", async () => {
    await expect(page.getByRole("link", { name: "Invitations" })).toBeVisible();
    await page.getByRole("link", { name: "Invitations" }).click();
    await expect(page.getByRole("listitem")).toBeVisible();
    await expect(page.locator("h3")).toContainText("Test team 2");
    await page.getByRole("button", { name: "Accept" }).click();
    await page.getByRole("link", { name: "Teams" }).click();
    await expect(page.getByRole("main")).toContainText("Test team 2");
  });
});
