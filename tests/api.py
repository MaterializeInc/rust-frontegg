#!/usr/bin/env python3
import os
import datetime
from unittest import SkipTest, TestCase, main
import uuid

import frontegg_api


class TestTenantsAndUsers(TestCase):
    """
    Rough-and-ready replica of test_tenants_and_users in the Rust API tests;
    driving the same functionality from both Rust and Python should show
    fidelity of the Python bindings.
    """

    TENANT_NAME_PREFIX = "test tenant"

    @classmethod
    def setUpClass(cls):
        cls.client = frontegg_api.Client(
            os.environ["FRONTEGG_CLIENT_ID"], os.environ["FRONTEGG_SECRET_KEY"]
        )

        cls.tenant_ids = [uuid.uuid4(), uuid.uuid4()]
        t1 = cls.client.create_tenant(
            id=cls.tenant_ids[0],
            name=f"{cls.TENANT_NAME_PREFIX} 1",
            metadata={"tenant_number": 1},
            creator_name="tenant 1",
            creator_email="creator@tenant1.com",
        )
        t2 = cls.client.create_tenant(
            id=cls.tenant_ids[1],
            name=f"{cls.TENANT_NAME_PREFIX} 2",
            metadata=42,
        )
        cls.tenants = [t1, t2]

        cls.users = []
        for tenant_idx, tenant_id in enumerate(cls.tenant_ids):
            name = f"user-{tenant_idx}-00"
            email = f"frontegg-test-{tenant_idx}-00@example.com"
            user = cls.client.create_user(
                tenant_id=tenant_id,
                name=name,
                email=email,
                skip_invite_email=True,
            )
            cls.users.append(user)

    def test_list_tenants(self):
        all_ts = self.client.list_tenants()
        test_tenants = [t for t in all_ts if t.name.startswith(self.TENANT_NAME_PREFIX)]
        self.assertEqual(len(test_tenants), 2)

    def test_create_tenant(self):
        t_id = uuid.uuid4()
        t = self.client.create_tenant(
            id=t_id,
            name=f"{self.TENANT_NAME_PREFIX} 3",
            metadata={"tenant_number": 3},
            creator_name="tenant 3",
            creator_email="creator@tenant3.com",
        )
        self.tenant_ids.append(t.id)
        self.tenants.append(t)
        self.assertEqual(t.id, t_id)
        self.assertEqual(t.name, f"{self.TENANT_NAME_PREFIX} 3")
        self.assertEqual(t.metadata, {"tenant_number": 3})
        self.assertEqual(t.creator_name, "tenant 3")
        self.assertEqual(t.creator_email, "creator@tenant3.com")
        self.assertEqual(t.deleted_at, None)

    def test_get_tenant(self):
        t = self.client.get_tenant(self.tenant_ids[1])
        self.assertEqual(t.name, f"{self.TENANT_NAME_PREFIX} 2")
        self.assertEqual(t.metadata, 42)
        self.assertEqual(t.creator_name, None)
        self.assertEqual(t.creator_email, None)

    def test_delete_tenant(self):
        self.client.delete_tenant(self.tenant_ids[2])
        with self.assertRaises(frontegg_api.NotFoundError):
            self.client.get_tenant(self.tenant_ids[2])

    def test_tenant_metadata(self):
        self.client.set_tenant_metadata(
            self.tenant_ids[0],
            {
                "tenant_name": self.tenants[0].name,
            },
        )
        tenant = self.client.get_tenant(self.tenants[0].id)
        self.assertEqual(
            tenant.metadata, {"tenant_name": tenant.name, "tenant_number": 1}
        )

        self.client.set_tenant_metadata(self.tenants[0].id, {"tenant_name": "set test"})
        tenant = self.client.get_tenant(self.tenants[0].id)
        self.assertEqual(
            tenant.metadata, {"tenant_name": "set test", "tenant_number": 1}
        )

        self.client.delete_tenant_metadata(self.tenants[0].id, "tenant_name")
        tenant = self.client.get_tenant(self.tenants[0].id)
        self.assertEqual(tenant.metadata, {"tenant_number": 1})

    def test_list_users(self):
        all_users = self.client.list_users()
        self.assertEqual(
            len([u for u in all_users if u.email.startswith("frontegg-test-")]), 7
        )
        all_users_paginated = self.client.list_users(page_size=1)
        self.assertEqual(len(all_users), len(all_users_paginated))
        one_tenant_users = self.client.list_users(tenant_id=self.tenant_ids[0])
        self.assertTrue(
            all([u.email.startswith("frontegg-test-0") for u in one_tenant_users])
        )
        one_tenant_users_uuid = self.client.list_users(tenant_id=self.tenant_ids[0])
        self.assertEqual(
            [u.id for u in one_tenant_users], [u.id for u in one_tenant_users_uuid]
        )

    def test_get_user(self):
        with self.assertRaises(ValueError):
            self.client.get_user("foo")
        with self.assertRaises(frontegg_api.NotFoundError):
            self.client.get_user(uuid.uuid4())
        known = self.client.get_user(self.users[0].id)
        self.assertIsInstance(known.created_at, datetime.datetime)
        self.assertIsInstance(known.id, uuid.UUID)
        self.assertEqual(known.name, f"user-0-00")
        self.assertEqual(known.email, "frontegg-test-0-00@example.com")
        self.assertEqual(known.metadata, None)

    def test_create_user(self):
        for tenant_idx, tenant_id in enumerate(self.tenant_ids):
            for user_idx in range(3):
                name = f"user-{tenant_idx}-{user_idx}"
                email = f"frontegg-test-{tenant_idx}-{user_idx}@example.com"
                created_user = self.client.create_user(
                    tenant_id=tenant_id,
                    name=name,
                    email=email,
                    skip_invite_email=True,
                )

                self.assertEqual(created_user.name, name)
                self.assertIsInstance(created_user.id, uuid.UUID)

    def test_delete_user(self):
        self.client.delete_user(self.users[-1].id)
        with self.assertRaises(frontegg_api.NotFoundError):
            self.client.get_user(self.users[-1].id)

    def test_user_roles(self):
        user = self.client.get_user(self.users[0].id)
        self.assertGreater(len(user.tenants), 0)
        binding_to_tenant_1 = [
            t for t in user.tenants if t.tenant_id == self.tenants[0].id
        ]
        self.assertEqual(len(binding_to_tenant_1), 1)
        self.assertEqual(binding_to_tenant_1[0].tenant_id, self.tenants[0].id)
        self.assertEqual(binding_to_tenant_1[0].roles, [])
        self.assertEqual(user.tenants[0].roles, [])

    # def test_get_workspace_client_id(self) -> str:
    #     raise SkipTest()

    # def test_verify_user(self):
    #     raise SkipTest()

    # def test_create_api_token(self):
    #    raise SkipTest()

    @classmethod
    def tearDownClass(cls):
        for tenant in cls.client.list_tenants():
            if tenant.name.startswith(cls.TENANT_NAME_PREFIX):
                cls.client.delete_tenant(str(tenant.id))


if __name__ == "__main__":
    main()
