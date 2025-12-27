import { useState } from 'react';
import { motion } from 'framer-motion';
import { PlusIcon, FunnelIcon } from '@heroicons/react/24/outline';
import { Button } from '../components/common/Button';
import { Modal } from '../components/common/Modal';
import { Input } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { Tabs, TabList, TabPanel, TabPanels } from '../components/common/Tabs';
import { ResourceGroupCard } from '../components/resources/ResourceGroupCard';
import { ResourceAllocationChart } from '../components/resources/ResourceAllocationChart';
import { ResourceGroupForm, ResourceGroupFormData } from '../components/resources/ResourceGroupForm';
import { DeleteConfirmDialog } from '../components/common/ConfirmDialog';
import { useResources } from '../hooks/useResources';
import { ResourceGroup } from '../types';

// ============================================================================
// Resource Groups Page
// Manage database resource groups and allocation
// ============================================================================

export default function ResourceGroups() {
  const {
    groups,
    usageMap,
    loading,
    createGroup,
    updateGroup,
    deleteGroup,
    refreshGroups,
  } = useResources();

  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingGroup, setEditingGroup] = useState<ResourceGroup | null>(null);
  const [deletingGroup, setDeletingGroup] = useState<ResourceGroup | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<'all' | 'enabled' | 'disabled'>('all');
  const [activeTab, setActiveTab] = useState('overview');

  // Filter groups
  const filteredGroups = groups.filter((group) => {
    const matchesSearch = group.name.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesStatus =
      filterStatus === 'all' ||
      (filterStatus === 'enabled' && group.isEnabled) ||
      (filterStatus === 'disabled' && !group.isEnabled);
    return matchesSearch && matchesStatus;
  });

  const handleCreateGroup = async (data: ResourceGroupFormData) => {
    await createGroup(data);
    setShowCreateModal(false);
  };

  const handleUpdateGroup = async (data: ResourceGroupFormData) => {
    if (editingGroup) {
      await updateGroup(editingGroup.id, data);
      setEditingGroup(null);
    }
  };

  const handleDeleteGroup = async () => {
    if (deletingGroup) {
      await deleteGroup(deletingGroup.id);
      setDeletingGroup(null);
    }
  };

  const tabs = [
    { id: 'overview', label: 'Overview', badge: groups.length },
    { id: 'allocation', label: 'Allocation' },
    { id: 'usage', label: 'Usage' },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-dark-100">Resource Groups</h1>
          <p className="text-dark-400 mt-1">
            Manage database resource allocation and limits
          </p>
        </div>
        <Button
          variant="primary"
          leftIcon={<PlusIcon className="w-5 h-5" />}
          onClick={() => setShowCreateModal(true)}
        >
          Create Resource Group
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <Input
          placeholder="Search resource groups..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="flex-1 max-w-md"
        />
        <Select
          options={[
            { value: 'all', label: 'All Status' },
            { value: 'enabled', label: 'Enabled Only' },
            { value: 'disabled', label: 'Disabled Only' },
          ]}
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value as 'all' | 'enabled' | 'disabled')}
          leftIcon={<FunnelIcon className="w-4 h-4" />}
        />
        <Button variant="ghost" onClick={refreshGroups}>
          Refresh
        </Button>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onChange={setActiveTab}>
        <TabList tabs={tabs} />

        <TabPanels className="mt-6">
          {/* Overview Tab */}
          <TabPanel tabId="overview">
            {loading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {[...Array(6)].map((_, i) => (
                  <div key={i} className="h-96 bg-dark-800 rounded-xl animate-pulse" />
                ))}
              </div>
            ) : filteredGroups.length === 0 ? (
              <div className="text-center py-12">
                <p className="text-dark-400 mb-4">
                  {searchQuery || filterStatus !== 'all'
                    ? 'No resource groups match your filters'
                    : 'No resource groups configured'}
                </p>
                <Button
                  variant="primary"
                  onClick={() => setShowCreateModal(true)}
                  leftIcon={<PlusIcon className="w-5 h-5" />}
                >
                  Create First Resource Group
                </Button>
              </div>
            ) : (
              <motion.div
                className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
              >
                {filteredGroups.map((group, index) => (
                  <motion.div
                    key={group.id}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.05 }}
                  >
                    <ResourceGroupCard
                      group={group}
                      usage={usageMap.get(group.id)}
                      onEdit={setEditingGroup}
                      onDelete={setDeletingGroup}
                    />
                  </motion.div>
                ))}
              </motion.div>
            )}
          </TabPanel>

          {/* Allocation Tab */}
          <TabPanel tabId="allocation">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <ResourceAllocationChart
                groups={groups}
                usageData={usageMap}
                resourceType="cpu"
              />
              <ResourceAllocationChart
                groups={groups}
                usageData={usageMap}
                resourceType="memory"
              />
              <ResourceAllocationChart
                groups={groups}
                usageData={usageMap}
                resourceType="connections"
              />
            </div>
          </TabPanel>

          {/* Usage Tab */}
          <TabPanel tabId="usage">
            <div className="bg-dark-800 rounded-xl p-6 text-center text-dark-400">
              Detailed usage analytics coming soon...
            </div>
          </TabPanel>
        </TabPanels>
      </Tabs>

      {/* Create Modal */}
      <Modal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        title="Create Resource Group"
        size="lg"
      >
        <ResourceGroupForm
          availableMembers={['admin', 'developer', 'analyst', 'viewer']}
          onSubmit={handleCreateGroup}
          onCancel={() => setShowCreateModal(false)}
        />
      </Modal>

      {/* Edit Modal */}
      <Modal
        isOpen={!!editingGroup}
        onClose={() => setEditingGroup(null)}
        title="Edit Resource Group"
        size="lg"
      >
        {editingGroup && (
          <ResourceGroupForm
            group={editingGroup}
            availableMembers={['admin', 'developer', 'analyst', 'viewer']}
            onSubmit={handleUpdateGroup}
            onCancel={() => setEditingGroup(null)}
          />
        )}
      </Modal>

      {/* Delete Confirmation */}
      <DeleteConfirmDialog
        isOpen={!!deletingGroup}
        onClose={() => setDeletingGroup(null)}
        onConfirm={handleDeleteGroup}
        itemName={deletingGroup?.name || ''}
        itemType="resource group"
        additionalWarning="All members will lose their resource limits."
      />
    </div>
  );
}
