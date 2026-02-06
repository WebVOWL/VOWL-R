use leptos::prelude::*;

#[component]
pub fn MegaMenu() -> impl IntoView {
    view! {
        <div
            id="mega-menu"
            class="hidden justify-between items-center w-full md:flex md:order-1 md:w-auto"
        >
            <ul class="flex flex-col mt-4 font-medium md:flex-row md:mt-0 md:space-x-8 rtl:space-x-reverse">
                <li>
                    <a
                        href="#"
                        class="block py-2 px-3 border-b md:p-0 md:border-0 text-fg-brand border-light md:hover:bg-transparent md:hover:text-fg-brand hover:bg-neutral-secondary-soft"
                        aria-current="page"
                    >
                        Home
                    </a>
                </li>
                <li>
                    <button
                        id="mega-menu-dropdown-button"
                        data-dropdown-toggle="mega-menu-dropdown"
                        class="flex justify-between items-center py-2 px-3 w-full font-medium border-b md:p-0 md:w-auto md:border-0 text-heading border-light md:hover:bg-transparent md:hover:text-fg-brand hover:bg-neutral-secondary-soft"
                    >
                        Company
                        <svg
                            class="w-4 h-4 ms-1.5"
                            aria-hidden="true"
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            fill="none"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke="currentColor"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="m19 9-7 7-7-7"
                            />
                        </svg>
                    </button>
                    <div
                        id="mega-menu-dropdown"
                        class="grid absolute z-10 grid-cols-2 w-auto text-sm rounded-xl border shadow md:grid-cols-3 bg-neutral-primary-soft border-default"
                    >
                        <div class="p-4 pb-0 md:pb-4 text-heading">
                            <ul
                                class="space-y-3"
                                aria-labelledby="mega-menu-dropdown-button"
                            >
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        About Us
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Library
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Resources
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Pro Version
                                    </a>
                                </li>
                            </ul>
                        </div>
                        <div class="p-4 pb-0 md:pb-4">
                            <ul class="space-y-3">
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Blog
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Newsletter
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Playground
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        License
                                    </a>
                                </li>
                            </ul>
                        </div>
                        <div class="p-4">
                            <ul class="space-y-3">
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Contact Us
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Support Center
                                    </a>
                                </li>
                                <li>
                                    <a href="#" class="text-body hover:text-fg-brand">
                                        Terms
                                    </a>
                                </li>
                            </ul>
                        </div>
                    </div>
                </li>
                <li>
                    <a
                        href="#"
                        class="block py-2 px-3 border-b md:p-0 md:border-0 text-heading border-light md:hover:bg-transparent md:hover:text-fg-brand hover:bg-neutral-secondary-soft"
                    >
                        Team
                    </a>
                </li>
                <li>
                    <a
                        href="#"
                        class="block py-2 px-3 border-b md:p-0 md:border-0 text-heading border-light md:hover:bg-transparent md:hover:text-fg-brand hover:bg-neutral-secondary-soft"
                    >
                        Contact
                    </a>
                </li>
            </ul>
        </div>
    }
}
